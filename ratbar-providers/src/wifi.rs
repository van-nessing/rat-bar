use crate::Provider;
use async_compat::CompatExt;
use itertools::*;
use netlink_wi::AsyncNlSocket;
use std::time::Duration;

pub struct Wifi {
    config: WifiArgs,
    connection: Option<Connection>,
    socket: AsyncNlSocket,
}
pub struct Connection {
    ssid: String,
    signal: i8,
}

#[derive(clap::Args)]
pub struct WifiArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    #[arg(long, default_values_t = ['󰤟', '󰤢', '󰤥', '󰤨'])]
    /// Symbols used for signal strength
    strength_symbols: Vec<char>,
    #[arg(long, default_value_t = '󰤭')]
    /// Symbol used when disconnected
    disconnected_symbol: char,
    #[arg(long, default_value_t = -80)]
    /// Minimum signal strength in dBm
    min_dbm: i8,
    #[arg(long, default_value_t = -67)]
    /// Maximum signal strength in dBm
    max_dbm: i8,
    /// Wireless interface to select
    interface: Option<String>,
}

#[derive(serde::Serialize)]
pub struct WifiFormat<'a> {
    ssid: &'a str,
    percentage: u8,
    dbm: i8,
    symbol: String,
}

impl Provider for Wifi {
    type Args = WifiArgs;
    type Fmt<'a> = WifiFormat<'a>;
    fn init(args: Self::Args) -> color_eyre::Result<Wifi> {
        Ok(Wifi {
            config: args,
            connection: None,
            socket: smol::block_on(AsyncNlSocket::connect().compat())?,
        })
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        smol::block_on(
            async {
                // love using async because the blocking version deadlocks after suspend!!
                let interfaces = self.socket.list_interfaces().await?;

                let mut interfaces = interfaces.into_iter();
                let interface = if let Some(interface) = self.config.interface.as_ref() {
                    interfaces.find_or_first(|i| i.name == interface.as_str())
                } else {
                    interfaces.next()
                };
                if let Some(interface) = interface {
                    let connection = self
                        .socket
                        .list_stations(interface.interface_index)
                        .await
                        .ok()
                        .and_then(|stations| {
                            let station = stations.first()?;
                            Some(Connection {
                                ssid: interface.ssid.clone()?,
                                signal: station.signal? as i8,
                            })
                        });
                    self.connection = connection
                }
                Ok(())
            }
            .compat(),
        )
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.config.duration)
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        let min = -self.config.min_dbm as i16;
        let max = -self.config.max_dbm as i16;
        let diff = min - max;
        if let Some(connection) = &self.connection {
            let db = (connection.signal as i16).clamp(-min, -max);
            let percentage = (db * 100 + min * 100) / diff;
            let fraction = 100 / (self.config.strength_symbols.len() - 1) as i16;
            let index = (percentage / fraction) as usize;
            let symbol = self.config.strength_symbols[index];
            Ok(WifiFormat {
                ssid: connection.ssid.as_ref(),
                percentage: percentage as u8,
                dbm: connection.signal,
                symbol: format!("{symbol} "),
            })
        } else {
            Ok(WifiFormat {
                ssid: "",
                percentage: 0,
                dbm: -100,
                symbol: format!("{} ", self.config.disconnected_symbol),
            })
        }
    }
}
