use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use clap::Parser;
use color_eyre::eyre::eyre;
use futures_concurrency::future::Race;

use crate::{app::App, components::BarComponent, config::Config, event::EventTask, ui::Ui};

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
    let (request_sender, request_receiver) = tokio::sync::mpsc::channel(32);

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
        component: BarComponent {
            constraint: ratatui::layout::Constraint::Fill(1),
            block: None,
            component_type: components::BarComponentType::Group {
                flex: ratatui::layout::Flex::SpaceAround,
                spacing: 0.into(),
                components: config.layout,
            },
        },
    };

    let app = App::new(running.clone(), event_receiver, request_sender, ui).await?;
    let dispatcher = EventTask::new(
        running.clone(),
        event_sender,
        request_receiver,
        config.providers,
    )?;

    let terminal = ratatui::init();

    let result = (app.run(terminal), dispatcher.run()).race().await;
    running.store(false, std::sync::atomic::Ordering::Relaxed);

    ratatui::restore();
    result
}
