use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use color_eyre::eyre::eyre;
use crossterm::event::{EnableMouseCapture, MouseEvent, MouseEventKind};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Position, Rect},
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    components::{
        diagnostics::DiagnosticsMeta,
        provider::{AccessCache, ProviderState},
    },
    event::{Event, Request},
    ui::Ui,
};

// #[derive(Debug)]
pub struct Meta {
    pub provider: ProviderState,
    pub diagnostics: DiagnosticsMeta,
}

pub struct App {
    pub ui: Ui,
    pub state: State,
    pub running: Arc<AtomicBool>,
    pub events: Receiver<Event>,
}

pub struct State {
    pub providers: ProviderState,
    pub requests: Sender<Request>,
    pub mouse: MouseState,
    pub iteration: u32,
}

pub struct Dragging {
    pub start: Position,
}

pub struct MouseState {
    pub events: Vec<MouseEvent>,
    pub dragging: Option<Dragging>,
    pub scroll_speed: f32,
}
impl Default for MouseState {
    fn default() -> Self {
        Self {
            events: Default::default(),
            dragging: Default::default(),
            scroll_speed: 1.0,
        }
    }
}
impl MouseState {
    pub fn capture_events(&mut self, area: Rect) -> Vec<MouseEvent> {
        let (captured, events) = self
            .events
            .drain(..)
            .partition(|event| area.contains(Position::new(event.column, event.row)));
        self.events = events;
        captured
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
            ui,
            events,
            running,
            state: State {
                providers: ProviderState::default(),
                requests,
                mouse: MouseState::default(),
                iteration: 0,
            },
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        let refresh_rate = Duration::from_secs(1) / 60;
        let mut last_render = Instant::now() - refresh_rate;

        while self.running.load(Ordering::Relaxed) {
            let render_now = Instant::now();
            // limit refresh rate
            if render_now.duration_since(last_render) > refresh_rate {
                // reset image access
                self.state
                    .providers
                    .images
                    .values_mut()
                    .for_each(|access| access.reset());

                terminal.draw(|frame| {
                    let [dbug, rest] = frame.area().layout(&Layout::horizontal(&[
                        Constraint::Length(6),
                        Constraint::Fill(1),
                    ]));
                    frame.render_widget(self.state.iteration.to_string(), dbug);
                    frame.render_widget(&mut self, rest);
                })?;

                // remove all image protocols from cache that weren't rendered this frame
                self.state
                    .providers
                    .images
                    .retain(|_, access| access.accessed());

                last_render = render_now;
                self.state.iteration += 1;
                // self.meta.diagnostics.render_time = render_now.elapsed();
            }

            // self.meta.diagnostics.total_ticks += 1;
            // self.meta.diagnostics.queued_events = self.events.len() as u64;

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
                            // self.meta.diagnostics.event_times.crossterm = event_now.elapsed();
                        }
                        crossterm::event::Event::Mouse(event) => match event.kind {
                            MouseEventKind::Down(_)
                            | MouseEventKind::ScrollUp
                            | MouseEventKind::ScrollDown
                            | MouseEventKind::Drag(_) => {
                                self.state.mouse.events.push(event);
                            }
                            _ => (),
                        },
                        _ => {}
                    }
                }
                Event::UpdateProviders { providers } => {
                    self.state.providers = providers;
                }
                Event::UpdateProvider { name, variables } => {
                    self.state.providers.variables.extend(
                        variables
                            .into_iter()
                            .map(|(k, v)| (format!("{name}.{k}"), v)),
                    );
                }
                Event::ImageLoaded { path, protocol } => {
                    self.state
                        .providers
                        .images
                        .insert(path, AccessCache::new(Some(protocol)));
                }
            }
            // self.meta.diagnostics.event_time = event_now.elapsed();
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
