use std::{
    collections::HashMap,
    io::Stdin,
    process::Stdio,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use color_eyre::eyre::eyre;
use futures_concurrency::future::Race;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::{
    app::App, components::visualizer::visualizer_events, config::Provider, event::EventTask,
    ui::Ui, widgets::bar_graph::BarGraph,
};

pub mod app;
pub mod components;
pub mod config;
pub mod dbus_integration;
pub mod event;
pub mod theme;
pub mod ui;
pub mod widgets;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (sender, receiver) = tokio::sync::mpsc::channel(32);

    let running = Arc::new(AtomicBool::new(true));
    let dir = dirs::config_local_dir()
        .ok_or_else(|| eyre!("couldn't find config directory"))?
        .join("rat-bar");
    let file = tokio::fs::read(dir.join("layout.yaml")).await?;
    let ui = Ui {
        component: serde_yaml::from_slice(&file)?,
    };
    let app = App::new(running.clone(), receiver, ui).await?;

    let file = tokio::fs::read(dir.join("providers.yaml")).await?;
    let providers = serde_yaml::from_slice(&file)?;
    // let providers = [(
    //     "Battery".to_string(),
    //     Provider {
    //         tick_duration: Some(Duration::from_secs(1)),
    //         command: vec![
    //             "nu".to_string(),
    //             "--stdin".to_string(),
    //             "~/.config/rat-bar/providers/battery.nu".to_string(),
    //         ],
    //     },
    // )]
    // .into_iter()
    // .collect::<HashMap<_, _>>();

    let dispatcher = EventTask::new(running.clone(), sender, providers);

    let terminal = ratatui::init();

    let result = (app.run(terminal), dispatcher.run()).race().await;
    running.store(false, std::sync::atomic::Ordering::Relaxed);

    ratatui::restore();
    result
}
