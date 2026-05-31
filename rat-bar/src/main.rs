use std::{
    path::PathBuf,
    process::Stdio,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use clap::Parser;
use color_eyre::eyre::eyre;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use futures_concurrency::future::Race;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{app::App, components::BarComponent, config::Config, event::run_event_tasks, ui::Ui};

pub mod app;
pub mod components;
pub mod config;
pub mod event;
pub mod theme;
pub mod ui;
pub mod widgets;

#[derive(clap::Parser)]
pub struct Args {
    #[arg(short, long)]
    providers: Option<PathBuf>,
    #[arg(short, long)]
    layout: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let (event_sender, event_receiver) = tokio::sync::mpsc::channel(32);
    let (request_sender, requests_receiver) = tokio::sync::mpsc::channel(32);

    let running = Arc::new(AtomicBool::new(true));

    let dir = dirs::config_local_dir()
        .ok_or_else(|| eyre!("couldn't find config directory"))?
        .join("rat-bar");

    let config = Config {
        layout: {
            let slice = if let Some(path) = &args.layout {
                tokio::fs::read(path).await?
            } else {
                tokio::fs::read(dir.join("layout.yaml")).await?
            };
            let deserializer = serde_yaml::Deserializer::from_slice(&slice);
            serde_yaml::with::singleton_map_recursive::deserialize(deserializer)?
        },
        providers: {
            let slice = if let Some(path) = &args.providers {
                tokio::fs::read(path).await?
            } else {
                tokio::fs::read(dir.join("providers.yaml")).await?
            };
            let deserializer = serde_yaml::Deserializer::from_slice(&slice);
            serde_yaml::with::singleton_map_recursive::deserialize(deserializer)?
        },
    };

    let ui = Ui {
        components: config.layout,
    };

    let app = App::new(running.clone(), event_receiver, request_sender, ui).await?;
    let dispatcher = run_event_tasks(
        running.clone(),
        event_sender,
        requests_receiver,
        config.providers,
    );

    let mut terminal = ratatui::init();

    let b = terminal.backend_mut();
    crossterm::execute!(b, EnableMouseCapture)?;

    let result = (app.run(&mut terminal), dispatcher).race().await;
    running.store(false, std::sync::atomic::Ordering::Relaxed);

    let b = terminal.backend_mut();
    crossterm::execute!(b, DisableMouseCapture)?;

    ratatui::restore();
    result
}
