use crate::{Provider, trunc};
use color_eyre as eyre;
use eyre::eyre::eyre;
use gfxinfo::active_gpu;
use std::time::Duration;
use sysinfo::{Component, Components, CpuRefreshKind, RefreshKind, System};

pub struct Gpu {
    duration: Duration,
    gpu: Box<dyn gfxinfo::GpuInfo>,
    load: u32,
    total_mem: u64,
    used_mem: u64,
    temp: u32,
    acc: Vec<f32>,
}

#[derive(clap::Args)]
pub struct GpuArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    /// Amount of values to store for graph display
    #[arg(long, short, default_value_t = 8)]
    count: usize,
}

#[derive(serde::Serialize)]
pub struct GpuFormat<'a> {
    load: f32,
    total_mem: f32,
    used_mem: f32,
    mem_percent: f32,
    temp: u32,
    load_acc: &'a [f32],
}

fn to_gb(bytes: u64) -> f32 {
    trunc((bytes as f64 / 2.0f64.powi(30)) as f32)
}

impl Provider for Gpu {
    type Args = GpuArgs;
    type Fmt<'a> = GpuFormat<'a>;
    fn init(args: Self::Args) -> eyre::Result<Gpu> {
        Ok(Gpu {
            duration: args.duration,
            gpu: active_gpu()
                .map_err(|e| eyre!("failed to get gpu").wrap_err(e.to_string()))?
                .info(),
            load: 0,
            temp: 0,
            total_mem: 0,
            used_mem: 0,
            acc: vec![0.0; args.count],
        })
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        let gpu = &*self.gpu;
        self.load = gpu.load_pct();
        self.total_mem = gpu.total_vram();
        self.used_mem = gpu.used_vram();
        self.temp = gpu.temperature();
        self.acc.remove(0);
        self.acc.push(self.load as f32);
        Ok(())
    }
    fn format(&self) -> color_eyre::Result<Self::Fmt<'_>> {
        Ok(GpuFormat {
            load: self.load as f32,
            total_mem: to_gb(self.total_mem),
            used_mem: to_gb(self.used_mem),
            mem_percent: (100 * self.used_mem / self.total_mem) as f32,
            temp: self.temp,
            load_acc: &self.acc,
        })
    }
}
