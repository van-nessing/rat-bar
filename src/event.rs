use futures_concurrency::future::Race;
use ratatui::crossterm::event::Event as CrosstermEvent;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::{
    signal::unix::{Signal, SignalKind, signal},
    sync::mpsc::Sender,
};
use tokio_stream::StreamExt;

use crate::components::{
    now_playing::{PlayerInfo, now_playing_events},
    provider::{Provider, ProviderMeta, provider_events},
    visualizer::visualizer_events,
};

#[derive(Debug)]
pub enum Event {
    Crossterm(CrosstermEvent),
    UpdatePlayers {
        players: HashMap<String, PlayerInfo>,
    },
    SendAudioSample {
        frequencies: Vec<f32>,
        sample_rate: u32,
    },
    UpdateProviders {
        providers: ProviderMeta,
    },
    UpdateProvider {
        name: String,
        variables: HashMap<String, Value>,
    },
}

/// A thread that handles reading crossterm events and emitting tick events on a regular schedule.
pub struct EventTask {
    sender: Sender<Event>,
    providers: HashMap<String, crate::config::Provider>,
    running: Arc<AtomicBool>,
}

impl EventTask {
    /// Constructs a new instance of [`EventThread`].
    pub fn new(
        running: Arc<AtomicBool>,
        sender: Sender<Event>,
        providers: HashMap<String, crate::config::Provider>,
    ) -> Self {
        Self {
            sender,
            running,
            providers,
        }
    }

    pub async fn run(self) -> color_eyre::Result<()> {
        (
            crossterm_events(self.sender.clone()),
            signal_events(self.sender.clone()),
            now_playing_events(self.sender.clone()),
            visualizer_events(self.sender.clone(), self.running.clone()),
            provider_events(self.sender.clone(), self.providers),
        )
            .race()
            .await
    }
}

async fn crossterm_events(sender: Sender<Event>) -> color_eyre::Result<()> {
    let mut crossterm_events = crossterm::event::EventStream::new();
    loop {
        let crossterm_event = crossterm_events.next();
        if let Some(Ok(event)) = crossterm_event.await {
            sender.send(Event::Crossterm(event)).await?;
        }
    }
}

async fn signal_events(sender: Sender<Event>) -> color_eyre::Result<()> {
    let mut signals = [
        signal(SignalKind::terminate())?,
        signal(SignalKind::hangup())?,
        signal(SignalKind::quit())?,
        signal(SignalKind::interrupt())?,
        signal(SignalKind::quit())?,
    ];
    let signals = signals.iter_mut().map(Signal::recv).collect::<Vec<_>>();
    signals
        .race()
        .await
        .ok_or_else(|| color_eyre::eyre::eyre!("signal error?"))
}
