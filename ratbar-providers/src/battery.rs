use crate::Provider;
use std::{fs::read_to_string, time::Duration};

pub struct Battery {
    duration: Duration,
    battery: String,
    full: u64,
    now: u64,
    status: String,
}

#[derive(clap::Args)]
pub struct BatteryArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    /// CPU temperature sensor name
    battery: String,
}

#[derive(serde::Serialize)]
pub struct BatteryFormat<'a> {
    charge: u8,
    status: &'a str,
}

impl Provider for Battery {
    type Args = BatteryArgs;
    type Fmt<'a> = BatteryFormat<'a>;
    fn init(args: Self::Args) -> color_eyre::Result<Battery> {
        Ok(Battery {
            duration: args.duration,
            battery: args.battery,
            status: String::new(),
            full: 0,
            now: 0,
        })
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        let now = read_to_string(format!(
            "/sys/class/power_supply/{}/charge_now",
            self.battery
        ))?;
        let full = read_to_string(format!(
            "/sys/class/power_supply/{}/charge_full",
            self.battery
        ))?;
        let status = read_to_string(format!("/sys/class/power_supply/{}/status", self.battery))?;

        self.now = now.trim().parse()?;
        self.full = full.trim().parse()?;
        self.status = status;

        Ok(())
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        let charge = ((self.now * 100).checked_div(self.full))
            .unwrap_or(0)
            .min(100) as u8;
        let status = &self.status;
        Ok(BatteryFormat { charge, status })
    }
}
