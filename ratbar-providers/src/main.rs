use crate::{
    clock::{Clock, ClockArgs},
    cpu::{Cpu, CpuArgs},
    // gpu::{Gpu, GpuArgs},
    media::{Media, MediaArgs},
    mem::{Mem, MemArgs},
    net::{Net, NetArgs},
    niri::{Niri, NiriArgs},
    visualizer::{Visualizer, VisualizerArgs},
};
use clap::Parser;
use color_eyre as eyre;
use serde::Serialize;
use std::{
    io::{self, Write as _},
    thread::sleep,
    time::{Duration, Instant},
};

mod clock;
mod cpu;
// mod gpu;
mod media;
mod mem;
mod net;
mod niri;
#[cfg(feature = "visualizer")]
mod visualizer;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let command = Command::parse();

    match command {
        Command::Clock(args) => Clock::init(args)?.run(),
        Command::Cpu(args) => Cpu::init(args)?.run(),
        // Command::Gpu(args) => Gpu::init(args)?.run(),
        Command::Media(args) => Media::init(args)?.run(),
        Command::Mem(args) => Mem::init(args)?.run(),
        Command::Net(args) => Net::init(args)?.run(),
        Command::Niri(args) => Niri::init(args)?.run(),
        Command::Visualizer(args) => Visualizer::init(args)?.run(),
    }
}

#[derive(clap::Parser)]
pub enum Command {
    Clock(ClockArgs),
    Cpu(CpuArgs),
    // Gpu(GpuArgs),
    Media(MediaArgs),
    Mem(MemArgs),
    Net(NetArgs),
    Niri(NiriArgs),
    Visualizer(VisualizerArgs),
}

pub trait Provider
where
    Self: Sized,
{
    type Args;
    type Fmt<'a>: Serialize
    where
        Self: 'a;
    fn init(args: Self::Args) -> eyre::Result<Self>;
    fn run(mut self) -> eyre::Result<()> {
        let mut stdout = io::stdout().lock();

        match self.duration() {
            Some(duration) => loop {
                let now = Instant::now();
                self.update()?;
                self.send(&mut stdout)?;
                sleep(duration.saturating_sub(now.elapsed()));
            },
            None => loop {
                self.update()?;
                self.send(&mut stdout)?;
            },
        }
    }
    fn update(&mut self) -> eyre::Result<()>;
    fn duration(&self) -> Option<Duration>;
    fn format<'a>(&'a self) -> eyre::Result<Self::Fmt<'a>>;
    fn send(&self, mut stdout: &mut io::StdoutLock) -> eyre::Result<()> {
        let fmt = self.format()?;
        serde_json::to_writer(&mut stdout, &fmt)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        Ok(())
    }
}
pub fn send(stdout: &mut io::StdoutLock, out: &str) -> eyre::Result<()> {
    writeln!(stdout, "{out}")?;
    stdout.flush()?;
    Ok(())
}

fn trunc(float: f32) -> f32 {
    (float * 10.0).trunc() / 10.0
}
