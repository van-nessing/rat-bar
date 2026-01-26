use std::borrow::Cow;

use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Spacing},
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::{
    app::{Meta, Record},
    widgets::{
        graph::GraphWidget,
        kv_bar::{KVBar, KVBarFormat, KVPair},
        percentage_bar::BlockPercentageBar,
    },
};

pub struct RAM<'a> {
    pub meta: &'a RamMeta,
}
pub struct RamMeta {
    pub total: u64,
    pub free: u64,
    pub used: Record<u64>,
}

#[derive(Debug)]
pub struct RamUpdate {
    pub used: u64,
    pub free: u64,
    pub total: u64,
}

impl RamMeta {
    pub fn update(&mut self, update: RamUpdate) {
        self.total = update.total;
        self.free = update.free;
        self.used.push_point(update.used);
    }
}

impl Widget for RAM<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let to_mb = 2u64.pow(20);
        let total_data = (self.meta.total / to_mb) as f32 / 1000.0;
        let used_data = (self.meta.used.datapoints().last().unwrap() / to_mb) as f32 / 1000.0;
        let free_data = (self.meta.free / to_mb) as f32 / 1000.0;

        let used_val = [Span::raw(format!("{used_data:.1}GB"))];
        let used = KVPair {
            key: "USED".into(),
            values: used_val.as_slice().into(),
        };
        let free_val = [Span::raw(format!("{free_data:.1}GB"))];
        let free = KVPair {
            key: "FREE".into(),
            values: free_val.as_slice().into(),
        };
        let total_val = [Span::raw(format!("{total_data:.1}GB"))];
        let total = KVPair {
            key: "TOTAL".into(),
            values: total_val.as_slice().into(),
        };
        // KVPair
        match area.height {
            1 => {
                let [used_val] = used_val;
                let [total_val] = total_val;
                let text = Line::from(vec![used_val, Span::raw("/"), total_val]);

                let graph = BlockPercentageBar {
                    style: Style::new(),
                    percentage: used_data,
                    direction: Direction::Horizontal,
                };

                let [text_area, graph_area] = area.layout(
                    &Layout::horizontal([
                        Constraint::Length(text.width() as u16),
                        Constraint::Percentage(100),
                    ])
                    .spacing(1),
                );
                graph.render(graph_area, buf);
                text.render(text_area, buf);
            }
            2 => {
                let pairs = [used, free];

                let text = KVBar {
                    pairs: pairs.as_slice().into(),
                    delimiter: None,
                    format: KVBarFormat::Horizontal { center: true },
                    spacing: 1,
                    show_keys: true,
                };
                let graph = BlockPercentageBar {
                    style: Style::new().on_dark_gray(),
                    percentage: used_data,
                    direction: Direction::Horizontal,
                };
                // let graph = GraphWidget {
                //     percentages: self.meta.ram_records.datapoints(),
                //     datapoint_count: self.meta.ram_records.max_points(),
                // };

                let [text_area, graph_area] = area.layout(
                    &Layout::horizontal([
                        Constraint::Length(text.width()),
                        Constraint::Percentage(100),
                    ])
                    .spacing(1),
                );
                text.render(text_area, buf);
                graph.render(graph_area, buf);
            }
            _ => {
                let pairs = [used, free, total];

                let text = KVBar {
                    pairs: pairs.as_slice().into(),
                    delimiter: Some(":".into()),
                    format: KVBarFormat::Vertical,
                    spacing: 1,
                    show_keys: true,
                };
                let values = self
                    .meta
                    .used
                    .datapoints()
                    .iter()
                    .map(|d| (d * 100) as f32 / self.meta.total as f32)
                    .collect::<Vec<_>>();
                let graph = GraphWidget {
                    percentages: values.as_slice(),
                    datapoint_count: self.meta.used.max_points(),
                };
                let [text_area, graph_area] = area.layout(&Layout::horizontal([
                    Constraint::Length(text.width()),
                    Constraint::Percentage(100),
                ]));
                text.render(text_area, buf);
                graph.render(graph_area, buf);
            }
        }
    }
}
