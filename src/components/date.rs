use chrono::{DateTime, Local};
use ratatui::{text::Span, widgets::Widget};
use serde::Deserialize;

use crate::widgets::kv_bar::{KVBar, KVBarFormat, KVPair};

#[derive(Debug, Deserialize)]
pub struct Date {
    pub elements: Vec<DateElement>,
}

#[derive(Debug, Deserialize)]
pub struct DateElement {
    pub name: String,
    pub fmt: String,
}

pub struct DateWidget<'a> {
    pub date: &'a Date,
    pub meta: &'a DateMeta,
}

#[derive(Debug)]
pub struct DateMeta {
    pub time: DateTime<Local>,
}

impl Default for Date {
    fn default() -> Self {
        Self {
            elements: vec![
                DateElement {
                    name: "DAY".into(),
                    fmt: "%a".into(),
                },
                DateElement {
                    name: "TIME".into(),
                    fmt: "%R".into(),
                },
                DateElement {
                    name: "DATE".into(),
                    fmt: "%d.%m.%Y".into(),
                },
            ],
        }
    }
}

impl Widget for DateWidget<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let elements = &self.date.elements;
        let pairs = elements
            .iter()
            .map(|elem| KVPair {
                key: Span::raw(elem.name.as_str()),
                values: vec![Span::from(self.meta.time.format(&elem.fmt).to_string())].into(),
            })
            .collect::<Vec<_>>();
        match area.height {
            0 => {}
            1 => {
                let bar = KVBar {
                    pairs: pairs.as_slice().into(),
                    format: KVBarFormat::Horizontal { center: true },
                    delimiter: None,
                    spacing: 1,
                    show_keys: false,
                };
                bar.render(area, buf);
            }
            2 => {
                let bar = KVBar {
                    pairs: pairs.as_slice().into(),
                    format: KVBarFormat::Horizontal { center: true },
                    delimiter: None,
                    spacing: 1,
                    show_keys: true,
                };
                bar.render(area, buf);
            }
            _ => {
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
