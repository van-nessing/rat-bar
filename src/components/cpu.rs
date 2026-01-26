use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Spacing},
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::{
    app::{Meta, Record},
    widgets::{
        bar_graph::BarGraph,
        graph::GraphWidget,
        kv_bar::{KVBar, KVBarFormat, KVPair},
        percentage_bar::BlockPercentageBar,
    },
};

pub struct CPU<'a> {
    pub meta: &'a CpuMeta,
}

pub struct CpuMeta {
    pub temp: f32,
    pub freq: u64,
    pub load: Record,
}

#[derive(Debug)]
pub struct CpuUpdate {
    pub freq: u64,
    pub temp: f32,
    pub load: f32,
}

impl CpuMeta {
    pub fn update(&mut self, update: CpuUpdate) {
        self.temp = update.temp;
        self.freq = update.freq;
        self.load.push_point(update.load);
    }
}

impl Widget for CPU<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let load = self.meta.load.datapoints().last().copied().unwrap_or(0.0);

        let temp = self.meta.temp;

        let freq = self.meta.freq as f32 / 1000.0;
        let load_data = load;
        let temp_data = temp;
        let freq_data = freq;

        let load_values = [Span::raw(format!("{load:.1}%"))];
        let load = KVPair {
            key: "LOAD".into(),
            values: load_values.as_slice().into(),
        };
        let freq_values = [Span::raw(format!("{freq:.1}Ghz"))];
        let freq = KVPair {
            key: "FREQ".into(),
            values: freq_values.as_slice().into(),
        };
        let temp_values = [Span::raw(format!("{temp:.0}°C"))];
        let temp = KVPair {
            key: "TEMP".into(),
            values: temp_values.as_slice().into(),
        };

        // KVPair
        match area.height {
            1 => {
                let pairs = [load];
                let text = KVBar {
                    // lines: lines.as_slice().into(),
                    pairs: pairs.as_slice().into(),
                    delimiter: Some(":".into()),
                    format: KVBarFormat::Inline,
                    spacing: 1,
                    show_keys: true,
                };

                let graph = BlockPercentageBar {
                    style: Style::new(),
                    percentage: load_data,
                    direction: Direction::Horizontal,
                };

                let [text_area, graph_area] = area.layout(
                    &Layout::horizontal([
                        Constraint::Length(text.width()),
                        Constraint::Percentage(100),
                    ])
                    .spacing(Spacing::Space(1)),
                );
                graph.render(graph_area, buf);
                text.render(text_area, buf);
            }
            2 => {
                let pairs = [load, freq];

                let text = KVBar {
                    pairs: pairs.as_slice().into(),
                    delimiter: None,
                    format: KVBarFormat::Horizontal { center: true },
                    spacing: 1,
                    show_keys: true,
                };
                let graph = GraphWidget {
                    percentages: self.meta.load.datapoints(),
                    datapoint_count: self.meta.load.max_points(),
                };

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
                let pairs = [load, freq, temp];

                let text = KVBar {
                    pairs: pairs.as_slice().into(),
                    delimiter: Some(":".into()),
                    format: KVBarFormat::Vertical,
                    spacing: 1,
                    show_keys: true,
                };
                let graph = GraphWidget {
                    percentages: self.meta.load.datapoints(),
                    datapoint_count: self.meta.load.max_points(),
                };
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
        }
    }
}
