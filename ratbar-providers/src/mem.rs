use std::time::Duration;

use crate::{Provider, trunc};
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

pub struct Mem {
    duration: Duration,
    system: System,
}

#[derive(clap::Args)]
pub struct MemArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
}

#[derive(serde::Serialize)]
pub struct MemFormat {
    used: f32,
    total: f32,
    available: f32,
    free: f32,
    percent: f32,
}

fn to_gb(bytes: u64) -> f32 {
    trunc((bytes as f64 / 2.0f64.powi(30)) as f32)
}

impl Provider for Mem {
    type Args = MemArgs;
    type Fmt<'a> = MemFormat;
    fn init(args: Self::Args) -> color_eyre::Result<Mem> {
        Ok(Mem {
            duration: args.duration,
            system: System::new_with_specifics(
                RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
            ),
        })
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        self.system.refresh_memory();
        Ok(())
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        let used = to_gb(self.system.used_memory());
        let total = to_gb(self.system.total_memory());
        let available = to_gb(self.system.available_memory());
        let free = to_gb(self.system.free_memory());
        let percent = (used / total) * 100.0;

        Ok(MemFormat {
            used,
            total,
            available,
            free,
            percent,
        })
    }
}
