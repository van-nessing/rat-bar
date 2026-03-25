use std::time::Duration;

use ratatui::{
    layout::{Constraint, Layout},
    text::Span,
    widgets::Widget,
};

use crate::{
    app::Meta,
    widgets::kv_bar::{KVBar, KVBarFormat, KVPair},
};

pub struct Diagnostics<'a> {
    pub meta: &'a DiagnosticsMeta,
}

#[derive(Debug, Default)]
pub struct DiagnosticsMeta {
    pub render_time: Duration,
    pub event_time: Duration,
    pub total_ticks: u64,
    pub queued_events: u64,
    pub event_times: EventTimes,
}
#[derive(Debug, Default)]
pub struct EventTimes {
    pub crossterm: Duration,
    pub sysinfo: Duration,
    pub niri: Duration,
    pub player: Duration,
    pub visualizer: Duration,
}
impl Widget for &Diagnostics<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let vals = [Span::raw(self.meta.total_ticks.to_string())];
        let ticks = KVPair {
            key: "TICKS".into(),
            values: vals.as_slice().into(),
        };

        let vals = [Span::raw(self.meta.queued_events.to_string())];
        let av_ticks = KVPair {
            key: "QUEUED".into(),
            values: vals.as_slice().into(),
        };

        let vals = [Span::raw(format!(
            "{:.2}ms",
            self.meta.render_time.as_secs_f32() * 1000.0
        ))];
        let render = KVPair {
            key: "RENDER".into(),
            values: vals.as_slice().into(),
        };

        let vals = [Span::raw(format!(
            "{:.2}ms",
            self.meta.event_time.as_secs_f32() * 1000.0
        ))];
        let events = KVPair {
            key: "EVENTS".into(),
            values: vals.as_slice().into(),
        };

        let vals = [Span::raw(format!(
            "{:.2}ms",
            self.meta.event_times.crossterm.as_secs_f32() * 1000.0
        ))];
        let crossterm = KVPair {
            key: "CROSS".into(),
            values: vals.as_slice().into(),
        };

        let vals = [Span::raw(format!(
            "{:.2}ms",
            self.meta.event_times.sysinfo.as_secs_f32() * 1000.0
        ))];
        let sys = KVPair {
            key: "SYS".into(),
            values: vals.as_slice().into(),
        };

        let vals = [Span::raw(format!(
            "{:.2}ms",
            self.meta.event_times.visualizer.as_secs_f32() * 1000.0
        ))];
        let vis = KVPair {
            key: "VIS".into(),
            values: vals.as_slice().into(),
        };

        let vals = [Span::raw(format!(
            "{:.2}ms",
            self.meta.event_times.niri.as_secs_f32() * 1000.0
        ))];
        let niri = KVPair {
            key: "NIRI".into(),
            values: vals.as_slice().into(),
        };

        let vals = [Span::raw(format!(
            "{:.2}ms",
            self.meta.event_times.player.as_secs_f32() * 1000.0
        ))];
        let song = KVPair {
            key: "SONG".into(),
            values: vals.as_slice().into(),
        };

        let pairs = [
            ticks, av_ticks, render, events, crossterm, sys, vis, niri, song,
        ];
        KVBar {
            pairs: pairs.as_slice().into(),
            format: KVBarFormat::Horizontal { center: true },
            delimiter: None,
            spacing: 1,
            show_keys: true,
        }
        .render(area, buf);
    }
}
