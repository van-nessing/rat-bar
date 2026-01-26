use std::{borrow::Cow, iter, ops::Deref};

use ratatui::{
    layout::{Constraint, Flex, Layout, Size, Spacing},
    text::{Line, Span},
    widgets::Widget,
};

pub struct KVBar<'a> {
    pub pairs: Cow<'a, [KVPair<'a>]>,
    pub format: KVBarFormat,
    pub delimiter: Option<Span<'a>>,
    pub spacing: u16,
    pub show_keys: bool,
}
#[derive(Clone)]
pub struct KVPair<'a> {
    pub key: Span<'a>,
    pub values: Cow<'a, [Span<'a>]>,
}
pub enum KVBarFormat {
    Inline,
    Horizontal { center: bool },
    Vertical,
}
impl KVBar<'_> {
    pub fn width(&self) -> u16 {
        match self.format {
            KVBarFormat::Inline => {
                let width = self
                    .pairs
                    .iter()
                    .flat_map(|pair| {
                        std::iter::once(
                            pair.key.width()
                                + self.delimiter.as_ref().map(|d| d.width()).unwrap_or(0),
                        )
                        .chain(pair.values.iter().map(Span::width))
                    })
                    .map(|i| i + self.spacing as usize)
                    .sum::<usize>() as u16;
                width
            }
            KVBarFormat::Horizontal { .. } => {
                let width = self
                    .pairs
                    .iter()
                    .map(|pair| {
                        if self.show_keys { pair.key.width() } else { 0 }
                            .max(pair.values.iter().map(Span::width).max().unwrap())
                            as u16
                    })
                    .sum::<u16>()
                    + (self.pairs.len() as u16) * self.spacing;
                width
            }
            KVBarFormat::Vertical => {
                let width = self
                    .pairs
                    .iter()
                    .map(|par| {
                        par.key.width() as u16
                            + par.values.iter().map(Span::width).sum::<usize>() as u16
                            + (self.pairs.len() as u16) * self.spacing
                    })
                    .max()
                    .unwrap();
                width
            }
        }
    }
}
impl Widget for &KVBar<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match self.format {
            KVBarFormat::Inline => {
                let constraints = self
                    .pairs
                    .iter()
                    .flat_map(|pair| {
                        std::iter::once(
                            pair.key.width()
                                + self.delimiter.as_ref().map(|d| d.width()).unwrap_or(0),
                        )
                        .chain(pair.values.iter().map(Span::width))
                    })
                    .map(|i| i as u16)
                    .map(Constraint::Length);

                let layout = Layout::horizontal(constraints).spacing(self.spacing);

                let layout = area.layout_vec(&layout);
                let mut layout = layout.into_iter();
                for pair in self.pairs.iter() {
                    let area = layout.next().unwrap();

                    if let Some(delimiter) = &self.delimiter {
                        Line::from(vec![pair.key.clone(), delimiter.clone()]).render(area, buf);
                    } else {
                        (&pair.key).render(area, buf);
                    }

                    for (span, area) in pair.values.iter().zip(&mut layout) {
                        span.render(area, buf);
                    }
                }
            }
            KVBarFormat::Vertical => {
                let span_count = self
                    .pairs
                    .iter()
                    .map(|pair| pair.values.len())
                    .max()
                    .unwrap();
                let delimiter_constraint = self.delimiter.as_ref().map(Span::width).unwrap_or(0);

                let key_constraints = iter::once(
                    self.pairs
                        .iter()
                        .map(|pair| pair.key.width() + delimiter_constraint)
                        .max()
                        .unwrap() as u16,
                );

                let value_constraints = (0..span_count).map(|i| {
                    self.pairs
                        .iter()
                        .map(move |pair| pair.values.get(i).map(Span::width).unwrap_or_default())
                        .max()
                        .unwrap() as u16
                });
                // .map(Constraint::Length);
                let constraints = key_constraints
                    .chain(value_constraints)
                    .map(Constraint::Length);

                let areas = area.layout_vec(&Layout::vertical(iter::repeat_n(
                    Constraint::Length(1),
                    self.pairs.len(),
                )));

                let layout = Layout::horizontal(constraints)
                    // .flex(self.flex)
                    .spacing(self.spacing);

                for (pair, area) in self.pairs.iter().zip(areas) {
                    let layout = area.layout_vec(&layout);
                    let mut layout = layout.into_iter();

                    if let Some(delimiter) = &self.delimiter {
                        Line::from(vec![pair.key.clone(), delimiter.clone()])
                            .render(layout.next().unwrap(), buf);
                    } else {
                        (&pair.key).render(layout.next().unwrap(), buf);
                    }

                    for (span, area) in pair.values.iter().zip(layout) {
                        span.render(area, buf);
                    }
                }
            }
            KVBarFormat::Horizontal { center } => {
                let constraints = self
                    .pairs
                    .iter()
                    .map(|pair| {
                        let keys = iter::once(&pair.key).chain(self.delimiter.as_ref());
                        self.show_keys
                            .then_some(keys)
                            .into_iter()
                            .flatten()
                            .chain(pair.values.iter())
                            .map(Span::width)
                            .max()
                            .unwrap()
                            .try_into()
                            .unwrap_or(u16::MAX)
                    })
                    .map(Constraint::Length);
                let areas = area.layout_vec(
                    &Layout::horizontal(constraints)
                        .spacing(self.spacing)
                        .flex(Flex::SpaceBetween),
                );

                let layout = Layout::vertical(iter::repeat_n(
                    Constraint::Length(1),
                    self.pairs
                        .iter()
                        .map(|pair| pair.values.len())
                        .max()
                        .unwrap()
                        + if self.show_keys { 1 } else { 0 },
                ));

                for (pair, area) in self.pairs.iter().zip(areas) {
                    let layout = area.layout_vec(&layout);
                    let mut layout = layout.into_iter();
                    if self.show_keys {
                        {
                            let mut area = layout.next().unwrap();
                            if center {
                                area = area.centered_horizontally(Constraint::Length(
                                    pair.key.width() as u16,
                                ));
                            }
                            (&pair.key).render(area, buf);
                        }

                        if let Some(delimiter) = &self.delimiter {
                            let mut area = layout.next().unwrap();
                            if center {
                                area = area.centered_horizontally(Constraint::Length(
                                    delimiter.width() as u16,
                                ))
                            }
                            delimiter.render(area, buf);
                        }
                    }
                    for (span, mut area) in pair.values.iter().zip(layout) {
                        if center {
                            area =
                                area.centered_horizontally(Constraint::Length(span.width() as u16));
                        }
                        span.render(area, buf);
                    }
                }
            }
        }
    }
}
