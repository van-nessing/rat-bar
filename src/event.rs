use async_channel::Sender;
use async_signal::{Signal, Signals};
use futures_concurrency::future::Race;
use futures_lite::StreamExt;
use sysinfo::{Components, Networks, RefreshKind, System};

use ratatui::crossterm::event::Event as CrosstermEvent;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use crate::components::{
    cpu::CpuUpdate,
    net::NetUpdate,
    niri_info::niri_events,
    now_playing::{PlayerInfo, now_playing_events},
    ram::RamUpdate,
    visualizer::visualizer_events,
};

#[derive(Debug)]
pub enum Event {
    Crossterm(CrosstermEvent),
    UpdateSysinfo {
        cpu: CpuUpdate,
        ram: RamUpdate,
        net: NetUpdate,
    },
    UpdateTime,
    UpdatePlayers {
        players: HashMap<String, PlayerInfo>,
    },
    NiriEvent {
        event: niri_ipc::Event,
    },
    SendAudioSample {
        frequencies: Vec<f32>,
        sample_rate: u32,
    },
}

#[derive(Debug)]
pub enum AppEvent {
    UpdateSysinfo {
        cpu: CpuUpdate,
        ram: RamUpdate,
        net: NetUpdate,
    },
    UpdateTime,
    AddPlayer {
        player: String,
        info: PlayerInfo,
    },
    UpdatePlayers {
        players: HashMap<String, PlayerInfo>,
    },
    NiriEvent {
        event: niri_ipc::Event,
    },
    SendAudioSample {
        frequencies: Vec<f32>,
        sample_rate: u32,
    },
    Error(color_eyre::Report),
    Quit,
}

/// A thread that handles reading crossterm events and emitting tick events on a regular schedule.
pub struct EventTask {
    sender: async_channel::Sender<Event>,
    running: Arc<AtomicBool>,
}

impl EventTask {
    /// Constructs a new instance of [`EventThread`].
    pub fn new(running: Arc<AtomicBool>, sender: async_channel::Sender<Event>) -> Self {
        Self { sender, running }
    }

    pub async fn run(self) -> color_eyre::Result<()> {
        (
            sysinfo_events(self.sender.clone(), "enp42s0"),
            crossterm_events(self.sender.clone()),
            signal_events(self.sender.clone()),
            now_playing_events(self.sender.clone()),
            visualizer_events(self.sender.clone(), self.running.clone()),
            niri_events(self.sender.clone()),
            time_events(self.sender.clone()),
        )
            .race()
            .await
    }
}
async fn sysinfo_events(sender: Sender<Event>, adapter: &str) -> color_eyre::Result<()> {
    let tick_rate = (Duration::from_secs(1)).max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    let mut timer = tokio::time::interval(tick_rate);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let refresh_kind = RefreshKind::everything().without_processes();

    let mut system = System::new_with_specifics(refresh_kind);
    let mut components = Components::new_with_refreshed_list();
    let mut net = Networks::new_with_refreshed_list();

    loop {
        timer.tick().await;
        let mut now_net;

        (system, components, net, now_net) = tokio::task::spawn_blocking(move || {
            system.refresh_specifics(refresh_kind);
            components.refresh(true);
            net.refresh(true);
            (system, components, net, Networks::new_with_refreshed_list())
        })
        .await?;

        let cpu = CpuUpdate {
            freq: system.cpus().iter().map(|cpu| cpu.frequency()).sum::<u64>()
                / system.cpus().len() as u64,
            temp: components
                .iter()
                .find(|c| c.label() == "k10temp Tctl")
                // .find(|c| c.label() == adapter)
                .and_then(|component| component.temperature())
                .unwrap_or(f32::NAN),
            load: system.global_cpu_usage(),
        };
        let ram = RamUpdate {
            used: system.used_memory(),
            free: system.available_memory(),
            total: system.total_memory(),
        };

        std::mem::swap(&mut net, &mut now_net);

        let net = NetUpdate {
            networks: now_net,
            refresh_duration: tick_rate,
        };

        sender.send(Event::UpdateSysinfo { cpu, ram, net }).await?;
    }
}

async fn time_events(sender: Sender<Event>) -> color_eyre::Result<()> {
    let tick_rate = Duration::from_secs(1);
    let mut timer = tokio::time::interval(tick_rate);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        timer.tick().await;
        sender.send(Event::UpdateTime).await?;
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
    let mut signals = Signals::new([Signal::Term, Signal::Quit, Signal::Int])?;
    signals.next().await;
    Ok(())
}
