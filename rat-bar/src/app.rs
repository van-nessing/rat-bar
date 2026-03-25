use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use color_eyre::eyre::eyre;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};
use ratatui_image::picker::Picker;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    components::{
        diagnostics::DiagnosticsMeta,
        provider::{AccessBuf, Provider, ProviderMeta},
    },
    event::{Event, Request},
    ui::Ui,
};

// #[derive(Debug)]
pub struct Meta {
    pub provider: ProviderMeta,
    pub diagnostics: DiagnosticsMeta,
}

#[derive(Debug)]
pub struct Record<T = f32> {
    max_points: usize,
    datapoints: Vec<T>,
}
pub struct App {
    pub ui: Ui,
    pub meta: Meta,
    pub picker: Picker,
    pub running: Arc<AtomicBool>,
    pub events: Receiver<Event>,
    pub requests: Sender<Request>,
}
impl<T: Default + Clone> Record<T> {
    pub fn new(max_points: usize) -> Self {
        Self {
            max_points,
            datapoints: vec![T::default(); max_points],
        }
    }
    pub fn push_point(&mut self, value: T) {
        self.datapoints.rotate_left(1);
        *self.datapoints.last_mut().unwrap() = value;
    }
    pub fn datapoints(&self) -> &[T] {
        &self.datapoints
    }
    pub fn max_points(&self) -> usize {
        self.max_points
    }
}
impl Default for Meta {
    fn default() -> Self {
        Self {
            provider: ProviderMeta {
                providers: HashMap::new(),
                images: HashMap::new(),
            },
            diagnostics: DiagnosticsMeta::default(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub async fn new(
        running: Arc<AtomicBool>,
        events: Receiver<Event>,
        requests: Sender<Request>,
        ui: Ui,
    ) -> color_eyre::Result<Self> {
        Ok(Self {
            meta: Default::default(),
            picker: Picker::from_query_stdio()?,
            ui,
            events,
            running,
            requests,
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let refresh_rate = Duration::from_secs(1) / 60;
        let mut last_render = Instant::now() - refresh_rate;

        while self.running.load(Ordering::Relaxed) {
            let render_now = Instant::now();
            // limit refresh rate
            if render_now.duration_since(last_render) > refresh_rate {
                // reset image access
                self.meta
                    .provider
                    .images
                    .values_mut()
                    .for_each(|access| access.reset());

                terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;

                // remove all image protocls from cache that weren't rendered this frame
                self.meta
                    .provider
                    .images
                    .retain(|_, access| access.accessed());

                last_render = render_now;
                self.meta.diagnostics.render_time = render_now.elapsed();
            }

            self.meta.diagnostics.total_ticks += 1;
            self.meta.diagnostics.queued_events = self.events.len() as u64;

            let event = self
                .events
                .recv()
                .await
                .ok_or_else(|| eyre!("channel closed"))?;

            let event_now = Instant::now();
            match event {
                Event::Crossterm(event) => {
                    let event_now = Instant::now();
                    match event {
                        crossterm::event::Event::Key(key_event)
                            if key_event.kind == crossterm::event::KeyEventKind::Press =>
                        {
                            self.handle_key_events(key_event)?;
                            self.meta.diagnostics.event_times.crossterm = event_now.elapsed();
                        }
                        _ => {}
                    }
                }
                Event::UpdateProviders { providers } => {
                    self.meta.provider = providers;
                }
                Event::UpdateProvider { name, variables } => {
                    self.meta
                        .provider
                        .providers
                        .entry(name)
                        .or_insert(Provider {
                            variables: Default::default(),
                        })
                        .update(variables);
                    // self.meta.provider
                    // self.meta.provider.providers.insert(name, provider);
                }
                Event::ImageLoaded { path, protocol } => {
                    self.meta
                        .provider
                        .images
                        .insert(path, AccessBuf::new(Some(protocol)));
                }
            }
            self.meta.diagnostics.event_time = event_now.elapsed();
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.quit(),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.quit();
            }
            _ => {}
        }
        Ok(())
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
