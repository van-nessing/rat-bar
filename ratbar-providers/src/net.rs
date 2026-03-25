use std::time::Duration;

use crate::{Provider, trunc};
use color_eyre::eyre::{OptionExt, eyre};
use sysinfo::Networks;

pub struct Net {
    duration: Duration,
    network: Networks,
    adapter: String,
}

#[derive(clap::Args)]
pub struct NetArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    /// Network adapter name
    adapter: Option<String>,
}

#[derive(serde::Serialize)]
pub struct NetFormat {
    sent: f32,
    recv: f32,
}

fn to_mb(bytes: u64) -> f32 {
    trunc((bytes as f64 / 2.0f64.powi(20)) as f32)
}

impl Provider for Net {
    type Args = NetArgs;
    type Fmt<'a> = NetFormat;
    fn init(args: Self::Args) -> color_eyre::Result<Net> {
        let network = Networks::new_with_refreshed_list();

        Ok(Net {
            duration: args.duration,
            adapter: args
                .adapter
                .or_else(|| {
                    network
                        .keys()
                        .find(|adapter| adapter.as_str() != "lo")
                        .cloned()
                })
                .ok_or_eyre("could not find network adapter")?,
            network,
        })
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        self.network.refresh(true);
        Ok(())
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        let adapter = self.network.get(&self.adapter);

        if let Some(adapter) = adapter {
            let sent = to_mb(adapter.transmitted());
            let recv = to_mb(adapter.received());
            Ok(NetFormat { sent, recv })
        } else {
            Err(eyre!("could not find adapter: '{}'", self.adapter))
        }
    }
}
