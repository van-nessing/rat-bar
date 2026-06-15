use crossterm::event::{Event as CrosstermEvent, MouseEvent};
use miette::IntoDiagnostic;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Position, Rect},
};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    event::{Event, Request},
    provider::{AccessCache, ProviderState},
    ui::Ui,
};

// #[derive(Debug)]
pub struct Meta {
    pub provider: ProviderState,
}

pub struct App {
    pub ui: Ui,
    pub state: State,
    pub running: bool,
    pub events: Receiver<Event>,
}

pub struct State {
    pub providers: ProviderState,
    pub requests: Sender<Request>,
    pub mouse: MouseState,
}

pub struct Dragging {
    pub start: Position,
}

pub struct MouseState {
    pub events: Vec<MouseEvent>,
    pub clicked: Option<Position>,
    pub scroll_speed: f32,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            events: Default::default(),
            clicked: Default::default(),
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
    pub fn is_click_in(&self, area: Rect) -> Option<Position> {
        self.clicked.filter(|click| area.contains(*click))
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub async fn new(
        events: Receiver<Event>,
        requests: Sender<Request>,
        ui: Ui,
    ) -> color_eyre::Result<Self> {
        Ok(Self {
            ui,
            running: true,
            events,
            state: State {
                providers: ProviderState::default(),
                requests,
                mouse: MouseState::default(),
            },
        })
    }

    pub async fn process_event(&mut self) -> miette::Result<()> {
        let event = self
            .events
            .recv()
            .await
            .ok_or_else(|| miette::miette!("channel closed"))?;

        match event {
            Event::Crossterm(CrosstermEvent::Key(key_event)) if key_event.is_press() => {
                self.handle_key_events(key_event)
            }
            Event::Crossterm(CrosstermEvent::Mouse(mouse_event)) if mouse_event.kind.is_down() => {
                self.state.mouse.clicked = Some(Position::new(mouse_event.column, mouse_event.row));
                self.state.mouse.events.push(mouse_event);
            }
            Event::Crossterm(CrosstermEvent::Mouse(mouse_event))
                if mouse_event.kind.is_scroll_up() | mouse_event.kind.is_scroll_down() =>
            {
                self.state.mouse.events.push(mouse_event);
            }
            Event::Crossterm(CrosstermEvent::Mouse(mouse_event)) if mouse_event.kind.is_up() => {
                self.state.mouse.clicked = None;
            }
            Event::Crossterm(_) => {}
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
        Ok(())
    }

    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> miette::Result<()> {
        let refresh_rate = Duration::from_secs(1) / 60;
        let mut last_render = Instant::now() - refresh_rate;

        while self.running {
            let render_now = Instant::now();
            // limit refresh rate
            if render_now.duration_since(last_render) > refresh_rate {
                self.state.providers.reset_image_access();

                terminal
                    .draw(|frame| frame.render_widget(&mut self, frame.area()))
                    .into_diagnostic()?;

                self.state.providers.remove_unused_image();

                last_render = render_now;
            }
            self.process_event().await?;
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.quit(),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.quit();
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false
    }
}
