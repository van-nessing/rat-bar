use crate::Provider;
use std::{collections::BTreeMap, time::Duration};

pub struct Clock {
    duration: Duration,
    expressions: Vec<(String, String)>,
}

#[derive(clap::Args)]
pub struct ClockArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    /// Expressions used get time in <key>=<val> format (example: day=%a time=%R)
    expressions: Vec<String>,
}

impl Provider for Clock {
    type Args = ClockArgs;
    type Fmt<'a> = BTreeMap<&'a str, String>;
    fn init(args: Self::Args) -> color_eyre::Result<Clock> {
        Ok(Clock {
            expressions: args
                .expressions
                .iter()
                .filter_map(|expr| expr.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            duration: args.duration,
        })
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        let local = chrono::Local::now();
        Ok(self
            .expressions
            .iter()
            .map(|(name, fmt)| (name.as_str(), local.format(fmt).to_string()))
            .collect::<BTreeMap<_, _>>())
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        Ok(())
    }
}
