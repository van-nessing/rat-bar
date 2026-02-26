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
use tokio::sync::mpsc::Receiver;

use crate::{
    components::{
        diagnostics::DiagnosticsMeta,
        now_playing::{NowPlayingMeta, PlayerInfo, SongMetadata},
        provider::ProviderMeta,
        visualizer::VisualizerMeta,
    },
    event::Event,
    ui::Ui,
};

// #[derive(Debug)]
pub struct Meta {
    pub provider: ProviderMeta,
    pub now_playing: NowPlayingMeta,
    pub visualizer: VisualizerMeta,
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
            },
            now_playing: NowPlayingMeta {
                players: Default::default(),
            },
            visualizer: VisualizerMeta::new(16, 256),
            diagnostics: DiagnosticsMeta::default(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub async fn new(
        running: Arc<AtomicBool>,
        events: Receiver<Event>,
        ui: Ui,
    ) -> color_eyre::Result<Self> {
        Ok(Self {
            meta: Default::default(),
            picker: Picker::from_query_stdio()?,
            ui,
            events,
            running,
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let refresh_rate = Duration::from_secs(1) / 30;
        let mut last_render = Instant::now() - refresh_rate;

        while self.running.load(Ordering::Relaxed) {
            let render_now = Instant::now();
            if render_now.duration_since(last_render) > refresh_rate {
                terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
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

                Event::UpdatePlayers { players } => {
                    for (id, player) in players.into_iter() {
                        let map = self
                            .meta
                            .now_playing
                            .players
                            .entry(id)
                            .or_insert(PlayerInfo {
                                name: player.name,
                                metadata: SongMetadata::default(),
                                state: player.state,
                            });
                        map.metadata.update(&self.picker, player.metadata).await?;
                        map.state = player.state;
                    }
                    self.meta.diagnostics.event_times.player = event_now.elapsed();
                }
                Event::SendAudioSample {
                    mut frequencies,
                    sample_rate,
                } => {
                    frequencies.truncate(128);
                    self.meta.visualizer.amp_average.rotate_right(1);
                    self.meta.visualizer.amp_average[0] =
                        frequencies.iter().sum::<f32>() / frequencies.len() as f32;

                    let scale = 0.9;
                    self.meta.visualizer.sample_rate = sample_rate;
                    self.meta.visualizer.data.rotate_right(1);
                    self.meta
                        .visualizer
                        .data
                        .iter_mut()
                        .for_each(|bins| bins.iter_mut().for_each(|bin| *bin *= scale));
                    self.meta.visualizer.data[0] = frequencies;
                    self.meta.diagnostics.event_times.visualizer = event_now.elapsed()
                }
                Event::UpdateProviders { providers } => {
                    self.meta.provider = providers;
                }
                Event::UpdateProvider { name, provider } => {
                    self.meta.provider.providers.insert(name, provider);
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
