use crate::Provider;
use std::{fs::read_to_string, time::Duration};

pub struct Battery {
    args: BatteryArgs,
    full: u64,
    now: u64,
    status: String,
}

#[derive(clap::Args)]
pub struct BatteryArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    /// Battery name
    battery: String,
    #[arg(long, value_parser, value_delimiter = ',', num_args = 1.., default_value = "󰂎 ,󰁺 ,󰁻 ,󰁼 ,󰁽 ,󰁾 ,󰁿 ,󰂀 ,󰂁 ,󰂂 ,󰁹 ")]
    /// Icons representing the charge
    charge_icons: Vec<String>,
    #[arg(long, value_parser, value_delimiter = ',', num_args = 1.., default_value = "󰢟 ,󰢜 ,󰂆 ,󰂇 ,󰂈 ,󰢝 ,󰂉 ,󰢞 ,󰂊 ,󰂋 ,󰂅 ")]
    /// Icons representing the charge when charging
    charging_charge_icons: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct BatteryFormat<'a> {
    charge: u8,
    icon: &'a str,
    status: &'a str,
}

impl Provider for Battery {
    type Args = BatteryArgs;
    type Fmt<'a> = BatteryFormat<'a>;
    fn init(args: Self::Args) -> color_eyre::Result<Battery> {
        Ok(Battery {
            args,
            status: String::new(),
            full: 100,
            now: 100,
        })
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.args.duration)
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        let now = read_to_string(format!(
            "/sys/class/power_supply/{}/charge_now",
            self.args.battery
        ))?;
        let full = read_to_string(format!(
            "/sys/class/power_supply/{}/charge_full",
            self.args.battery
        ))?;
        let status = read_to_string(format!(
            "/sys/class/power_supply/{}/status",
            self.args.battery
        ))?;

        self.now = now.trim().parse()?;
        self.full = full.trim().parse()?;
        self.status = status;

        Ok(())
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        let charge = ((self.now * 100).checked_div(self.full))
            .unwrap_or(0)
            .min(100) as u8;
        let status = &self.status.trim();
        let icon = match status.to_lowercase().as_str() {
            "charging" => {
                let idx = ((self.args.charging_charge_icons.len() - 1) * charge as usize) / 100;
                &self.args.charging_charge_icons[idx]
            }
            _ => {
                let idx = ((self.args.charge_icons.len() - 1) * charge as usize) / 100;
                &self.args.charge_icons[idx]
            }
        };
        Ok(BatteryFormat {
            charge,
            icon,
            status,
        })
    }
}
