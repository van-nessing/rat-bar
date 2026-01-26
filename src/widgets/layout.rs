use std::borrow::Cow;

use ratatui::{
    layout::{Constraint, Layout, Rect, Size},
    text::Line,
    widgets::Widget,
};

pub struct LayoutWidget<'a> {
    layout: LayoutVariant<'a>,
    variable_map: StackMap<'a, 6>,
}

pub enum LayoutVariant<'a> {
    Horizontal(Vec<LayoutVariant<'a>>),
    Vertical(Vec<LayoutVariant<'a>>),
    Element(LayoutElement<'a>),
}
pub enum LayoutElement<'a> {
    Str(Cow<'a, str>),
    Val(Cow<'a, str>),
}
pub enum LayoutElementWidget {}
impl LayoutElement<'_> {
    pub fn width(&self) -> usize {
        match self {
            LayoutElement::Str(cow) => cow.len(),
            LayoutElement::Val(cow) => cow.len(),
        }
    }
}

impl LayoutVariant<'_> {
    pub fn length(&self) -> Size {
        match self {
            Self::Horizontal(layouts) => {
                layouts
                    .iter()
                    .map(LayoutVariant::length)
                    .fold(Size::new(0, 0), |a, b| Size {
                        width: a.width + b.width,
                        height: a.height.max(b.height),
                    })
            }
            LayoutVariant::Vertical(layouts) => {
                layouts
                    .iter()
                    .map(LayoutVariant::length)
                    .fold(Size::new(0, 0), |a, b| Size {
                        height: a.height + b.height,
                        width: a.width.max(b.width),
                    })
            }
            LayoutVariant::Element(line) => Size {
                width: line.width().min(u16::MAX as usize) as u16,
                height: 1,
            },
        }
    }
}
impl Widget for &LayoutVariant<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match self {
            LayoutVariant::Horizontal(variants) => {
                let constraints = variants
                    .iter()
                    .map(LayoutVariant::length)
                    .map(|size| size.width)
                    .map(Constraint::Length);
                let layout = Layout::horizontal(constraints);
                let areas = area.layout_vec(&layout);
                let mapped = variants.iter().zip(areas.into_iter());
                for (variant, area) in mapped {
                    variant.render(area, buf);
                }
            }
            LayoutVariant::Vertical(variants) => {
                let constraints = variants
                    .iter()
                    .map(LayoutVariant::length)
                    .map(|size| size.height)
                    .map(Constraint::Length);
                let layout = Layout::vertical(constraints);
                let areas = area.layout_vec(&layout);
                let mapped = variants.iter().zip(areas.into_iter());
                for (variant, area) in mapped {
                    variant.render(area, buf);
                }
            }
            LayoutVariant::Element(element) => match element {
                LayoutElement::Str(cow) => {}
                LayoutElement::Val(cow) => todo!(),
            },
        }
    }
}

pub struct StackMap<'a, const N: usize> {
    keys: [Option<Cow<'a, str>>; N],
    vals: [Option<Cow<'a, str>>; N],
    elements: usize,
}

impl<'a, const N: usize> StackMap<'a, N> {
    pub fn get(&self, key: &str) -> Option<&str> {
        let position = self
            .keys
            .iter()
            .position(|k| k.as_deref().is_some_and(|k| *k == *key))?;
        self.vals[position].as_deref()
    }
    pub fn insert(&mut self, key: Cow<'a, str>, val: Cow<'a, str>) -> color_eyre::Result<usize> {
        if self.elements == N {
            return Err(color_eyre::eyre::eyre!("stack map full"));
        }
        self.keys[self.elements] = Some(key);
        self.vals[self.elements] = Some(val);

        let result = Ok(self.elements);
        self.elements += 1;

        result
    }
}
