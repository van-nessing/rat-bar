use std::sync::{Arc, atomic::AtomicBool};

use futures_concurrency::future::Race;

use crate::{
    app::App, components::visualizer::visualizer_events, event::EventTask,
    widgets::bar_graph::BarGraph,
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

    let (sender, receiver) = async_channel::unbounded();

    let running = Arc::new(AtomicBool::new(true));
    let app = App::new(running.clone(), receiver).await?;
    let dispatcher = EventTask::new(running.clone(), sender);

    let terminal = ratatui::init();

    let result = (app.run(terminal), dispatcher.run()).race().await;
    running.store(false, std::sync::atomic::Ordering::Relaxed);

    ratatui::restore();
    result
}
