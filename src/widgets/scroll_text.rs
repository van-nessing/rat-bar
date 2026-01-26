use std::time::{Duration, Instant};

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

#[derive(Debug)]
pub struct ScrollText<'a> {
    pub line: Line<'a>,
}

#[derive(Debug)]
pub struct ScrollTextState {
    offset: u16,
    scroll_interval: Duration,
    rest_duration: Duration,
    last_scroll: Instant,
}
impl Default for ScrollTextState {
    fn default() -> Self {
        Self {
            offset: Default::default(),
            scroll_interval: Duration::from_secs(1) / 8,
            rest_duration: Duration::from_secs(1),
            last_scroll: Instant::now(),
        }
    }
}
impl ScrollTextState {
    pub fn new(interval: Duration, rest: Duration) -> Self {
        Self {
            offset: 0,
            scroll_interval: interval,
            rest_duration: rest,
            last_scroll: Instant::now(),
        }
    }
    pub fn tick(&mut self, line: &Line<'_>, area: Rect, now: Instant) {
        let elapsed = now.duration_since(self.last_scroll);
        if line.width() as u16 <= area.width {
            self.offset = 0;
            self.last_scroll = now;
            return;
        }
        match self.offset {
            0 => {
                if elapsed > self.rest_duration {
                    self.offset += 1;
                    self.last_scroll = now;
                }
            }
            o if line.width() as u16 - o <= area.width => {
                if elapsed > self.rest_duration {
                    self.offset = 0;
                    self.last_scroll = now;
                }
            }
            _ => {
                if elapsed > self.scroll_interval {
                    self.offset += 1;
                    self.last_scroll = now;
                }
            }
        }
    }
}
impl StatefulWidget for &ScrollText<'_> {
    type State = ScrollTextState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut ScrollTextState,
    ) {
        let now = Instant::now();
        state.tick(&self.line, area, now);
        let mut offset = state.offset as usize;
        let spans = self.line.spans.iter().filter_map(|span| {
            let width = span.width();
            if offset == 0 {
                return Some(span.clone());
            }
            if let Some(rest) = offset.checked_sub(width) {
                offset = rest;
                return None;
            }
            let result = Some(
                Span::raw(span.content.chars().skip(offset).collect::<String>()).style(span.style),
            );
            offset = 0;
            result
        });
        let line = Line::from_iter(spans).style(self.line.style);
        line.render(area, buf);
    }
}
