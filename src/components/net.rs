use std::time::Duration;

use ratatui::{
    layout::{Constraint, Layout},
    text::Span,
    widgets::Widget,
};
use sysinfo::Networks;

use crate::{
    app::Meta,
    widgets::kv_bar::{KVBar, KVBarFormat, KVPair},
};

pub struct Net<'a> {
    pub adapter: &'a str,
    pub meta: &'a NetMeta,
}

pub struct NetMeta {
    pub networks: Networks,
    pub refresh_rate: Duration,
}

#[derive(Debug)]
pub struct NetUpdate {
    pub networks: Networks,
    pub refresh_duration: Duration,
}

impl NetMeta {
    pub fn update(&mut self, update: NetUpdate) {
        self.networks = update.networks;
        self.refresh_rate = update.refresh_duration;
    }
}

impl Widget for &Net<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let adapter = self.meta.networks.get(self.adapter);
        if let Some(adapter) = adapter {
            let refresh_rate = self.meta.refresh_rate.as_secs_f32();

            let to_kb = 2u64.pow(10);
            let tx = (adapter.transmitted() / to_kb) as f32 / 1000.0;
            let rx = (adapter.received() / to_kb) as f32 / 1000.0;

            let tx = tx / refresh_rate;
            let rx = rx / refresh_rate;

            let rx_values = [Span::raw(format!("{rx:>.2}MB/s"))];
            let rx = KVPair {
                key: "RX".into(),
                values: rx_values.as_slice().into(),
            };
            let tx_values = [Span::raw(format!("{tx:>.2}MB/s"))];
            let tx = KVPair {
                key: "TX".into(),
                values: tx_values.as_slice().into(),
            };

            match area.height {
                1 => {
                    let pairs = [rx, tx];
                    let bar = KVBar {
                        pairs: pairs.as_slice().into(),
                        format: KVBarFormat::Inline,
                        delimiter: Some(":".into()),
                        spacing: 1,
                        show_keys: true,
                    };
                    bar.render(area, buf);
                }
                2 | _ => {
                    let pairs = [rx, tx];
                    let bar = KVBar {
                        pairs: pairs.as_slice().into(),
                        format: KVBarFormat::Vertical,
                        delimiter: Some(":".into()),
                        spacing: 1,
                        show_keys: true,
                    };
                    bar.render(area, buf);
                }
            }
        }
    }
}
