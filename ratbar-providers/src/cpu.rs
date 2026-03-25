use crate::{Provider, trunc};
use std::time::Duration;
use sysinfo::{Component, Components, CpuRefreshKind, RefreshKind, System};

pub struct Cpu {
    duration: Duration,
    system: System,
    sensor: Option<Component>,
    acc: Vec<f32>,
}

#[derive(clap::Args)]
pub struct CpuArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    /// CPU temperature sensor name
    temp_sensor: String,
    /// Amount of values to store for graph display
    #[arg(long, short, default_value_t = 8)]
    count: usize,
}

#[derive(serde::Serialize)]
pub struct CpuFormat<'a> {
    load: f32,
    freq: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temp: Option<f32>,
    acc: &'a [f32],
}

fn to_ghz(mhz: u64) -> f32 {
    trunc((mhz as f64 / 1000.0) as f32)
}

impl Provider for Cpu {
    type Args = CpuArgs;
    type Fmt<'a> = CpuFormat<'a>;
    fn init(args: Self::Args) -> color_eyre::Result<Cpu> {
        Ok(Cpu {
            duration: args.duration,
            system: System::new_with_specifics(
                RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
            ),
            sensor: Vec::from(Components::new_with_refreshed_list())
                .into_iter()
                .find(|c| c.label() == args.temp_sensor),
            acc: vec![0.0; args.count],
        })
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        self.system.refresh_cpu_all();
        self.sensor.as_mut().map(Component::refresh);
        let load = trunc(self.system.global_cpu_usage());
        self.acc.remove(0);
        self.acc.push(load);
        Ok(())
    }
    fn format(&self) -> color_eyre::Result<Self::Fmt<'_>> {
        let cpus = self.system.cpus();
        let freq = to_ghz(cpus.iter().map(|cpu| cpu.frequency()).sum::<u64>() / cpus.len() as u64);
        let load = trunc(self.system.global_cpu_usage());
        let temp = self.sensor.as_ref().and_then(|c| c.temperature());

        Ok(CpuFormat {
            freq,
            load,
            temp,
            acc: &self.acc,
        })
    }
}
