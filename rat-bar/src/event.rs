use color_eyre::eyre::Context;
use futures_concurrency::future::Race;
use image::load_from_memory;
use ratatui::{crossterm::event::Event as CrosstermEvent, layout::Size};
use ratatui_image::{FilterType, Resize, picker::Picker, protocol::Protocol};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::{
    signal::unix::{Signal, SignalKind, signal},
    sync::mpsc::{Receiver, Sender},
};
use tokio_stream::StreamExt;

use crate::components::provider::{Provider, ProviderMeta, provider_events};

pub enum Event {
    Crossterm(CrosstermEvent),
    UpdateProviders {
        providers: ProviderMeta,
    },
    UpdateProvider {
        name: String,
        variables: HashMap<String, Value>,
    },
    ImageLoaded {
        path: String,
        protocol: Protocol,
    },
}

pub enum Request {
    LoadImage { path: String, size: Size },
}

/// A thread that handles reading crossterm events and emitting tick events on a regular schedule.
pub struct EventTask {
    sender: Sender<Event>,
    requests: Receiver<Request>,
    providers: HashMap<String, crate::config::Provider>,
    running: Arc<AtomicBool>,
    picker: Picker,
}

impl EventTask {
    /// Constructs a new instance of [`EventThread`].
    pub fn new(
        running: Arc<AtomicBool>,
        sender: Sender<Event>,
        requests: Receiver<Request>,
        providers: HashMap<String, crate::config::Provider>,
    ) -> color_eyre::Result<Self> {
        Ok(Self {
            sender,
            running,
            requests,
            providers,
            picker: Picker::from_query_stdio()?,
        })
    }

    pub async fn run(self) -> color_eyre::Result<()> {
        (
            crossterm_events(self.sender.clone()),
            signal_events(self.sender.clone()),
            provider_events(self.sender.clone(), self.providers),
            handle_requests(self.sender.clone(), self.requests, self.picker),
        )
            .race()
            .await
    }
}

async fn handle_requests(
    sender: Sender<Event>,
    mut requests: Receiver<Request>,
    picker: Picker,
) -> color_eyre::Result<()> {
    while let Some(request) = requests.recv().await {
        let picker = picker.clone();
        let sender = sender.clone();
        match request {
            Request::LoadImage { path, size } => {
                tokio::task::spawn(async move {
                    let image = tokio::fs::read(&path)
                        .await
                        .wrap_err_with(|| format!("error loading image: {path}"))?;
                    let image = tokio::task::spawn_blocking(move || {
                        load_from_memory(&image) //.map(|i| i.thumbnail(128, 128))
                    })
                    .await
                    .wrap_err_with(|| format!("error loading image: {path}"))??;
                    let protocol = Some(image).and_then(|image| {
                        picker
                            .new_protocol(image, size.into(), Resize::Scale(None))
                            .ok()
                    });
                    if let Some(protocol) = protocol {
                        sender.send(Event::ImageLoaded { path, protocol }).await?;
                    }
                    color_eyre::Result::<()>::Ok(())
                });
            }
        }
    }
    Ok(())
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
