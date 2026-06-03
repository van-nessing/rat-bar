use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use clap::Parser;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use futures_concurrency::future::Race;
use miette::IntoDiagnostic;

use crate::{app::App, config::Config, event::run_event_tasks, ui::Ui};

pub mod app;
pub mod config;
pub mod event;
pub mod layout;
pub mod provider;
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
async fn main() -> miette::Result<()> {
    // color_eyre::install()?;

    let args = Args::parse();

    let config = Config::load(args.providers.as_deref(), args.layout.as_deref())?;

    let ui = Ui {
        components: config.layout,
    };

    let (event_sender, event_receiver) = tokio::sync::mpsc::channel(32);
    let (request_sender, requests_receiver) = tokio::sync::mpsc::channel(32);

    let app = App::new(event_receiver, request_sender, ui).await.unwrap();
    let dispatcher = run_event_tasks(event_sender, requests_receiver, config.providers);

    let mut terminal = ratatui::init();

    crossterm::execute!(terminal.backend_mut(), EnableMouseCapture).into_diagnostic()?;

    let result = (app.run(&mut terminal), dispatcher).race().await;

    crossterm::execute!(terminal.backend_mut(), DisableMouseCapture).into_diagnostic()?;

    ratatui::restore();
    result
}
