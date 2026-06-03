use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};

use ratatui::layout::Constraint as RatConstraint;
use ratatui::{layout::Direction, widgets::StatefulWidget};

use crate::{
    app::State,
    widgets::{bar_graph::Marker, scroll_text::ScrollTextState},
};

#[derive(knuffel::Decode)]
pub enum Element {
    Group(Group),
    Text(Text),
    Bar(Bar),
    Graph(Graph),
    Image(Image),
}
#[derive(knuffel::Decode)]
pub struct Group {
    #[knuffel(argument, str)]
    direction: Direction,
    #[knuffel(child, unwrap(property, str))]
    width: Option<Constraint>,
    #[knuffel(child, unwrap(property, str), default)]
    center: bool,
    #[knuffel(child, unwrap(property))]
    on_click: Option<String>,
    #[knuffel(child, unwrap(property))]
    on_scroll: Option<String>,
    #[knuffel(children)]
    elements: Vec<Element>,
}
#[derive(knuffel::Decode)]
pub struct Image {
    #[knuffel(child, unwrap(property, str))]
    width: Constraint,
    #[knuffel(child, unwrap(property))]
    var: String,
    #[knuffel(child, unwrap(property))]
    on_click: Option<String>,
    #[knuffel(child, unwrap(property))]
    on_scroll: Option<String>,
}
#[derive(knuffel::Decode)]
pub struct Text {
    #[knuffel(argument)]
    string: String,
    #[knuffel(child, unwrap(property, str))]
    width: Option<Constraint>,
    #[knuffel(child, unwrap(property, str), default = false)]
    scroll: bool,
    #[knuffel(child, unwrap(property))]
    on_click: Option<String>,
    #[knuffel(child, unwrap(property))]
    on_scroll: Option<String>,
    scroll_state: ScrollTextState,
}
#[derive(knuffel::Decode)]
pub struct Bar {
    #[knuffel(child, unwrap(property, str))]
    width: Constraint,
    #[knuffel(child, unwrap(property, str))]
    direction: Direction,
    #[knuffel(child, unwrap(property))]
    var: String,
    #[knuffel(child, unwrap(property))]
    fg: String,
    #[knuffel(child, unwrap(property))]
    bg: String,
    #[knuffel(child, unwrap(property))]
    on_click: Option<String>,
    #[knuffel(child, unwrap(property))]
    on_scroll: Option<String>,
}
#[derive(knuffel::Decode)]
pub struct Graph {
    #[knuffel(child, unwrap(property, str))]
    width: Constraint,
    #[knuffel(child, unwrap(property, str))]
    marker: Option<Marker>,
    #[knuffel(child, unwrap(property))]
    var: String,
    #[knuffel(child, unwrap(property))]
    fg: String,
    #[knuffel(child, unwrap(property))]
    bg: String,
    #[knuffel(child, unwrap(property))]
    on_click: Option<String>,
    #[knuffel(child, unwrap(property))]
    on_scroll: Option<String>,
}

#[derive(Clone, Copy)]
pub enum Constraint {
    Length(u16),
    Percent(u16),
}
impl Default for Constraint {
    fn default() -> Self {
        Constraint::Percent(100)
    }
}
impl From<Constraint> for RatConstraint {
    fn from(value: Constraint) -> Self {
        match value {
            Constraint::Length(length) => RatConstraint::Length(length),
            Constraint::Percent(percent) => RatConstraint::Percentage(percent),
        }
    }
}
impl FromStr for Constraint {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Some(s) = s.strip_suffix("%") {
            Self::Percent(s.parse()?)
        } else {
            Self::Length(s.parse()?)
        })
    }
}

impl StatefulWidget for &mut Element {
    type State = State;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match self {
            Element::Group(Group {
                direction,
                width: constraint,
                center,
                on_click,
                on_scroll,
                elements,
            }) => {}
            Element::Text(Text {
                string,
                width,
                scroll,
                scroll_state,
                on_click,
                on_scroll,
            }) => todo!(),
            Element::Bar(Bar {
                width,
                direction,
                var,
                fg,
                bg,
                on_click,
                on_scroll,
            }) => todo!(),
            Element::Graph(Graph {
                width,
                marker,
                var,
                fg,
                bg,
                on_click,
                on_scroll,
            }) => todo!(),
            Element::Image(Image {
                width,
                var,
                on_click,
                on_scroll,
            }) => todo!(),
        }
    }
}

impl Element {
    pub fn height(&self) -> RatConstraint {
        match self {
            Element::Group(_group) => RatConstraint::Fill(1),
            Element::Text(_text) => RatConstraint::Length(1),
            Element::Bar(Bar { direction, .. }) => match direction {
                Direction::Horizontal => RatConstraint::Length(1),
                Direction::Vertical => RatConstraint::Fill(1),
            },
            Element::Graph(_graph) => RatConstraint::Fill(1),
            Element::Image(_image) => RatConstraint::Fill(1),
        }
    }
    pub fn width(&self, state: &State) -> RatConstraint {
        match self {
            Element::Group(group) => match (group.direction, group.width) {
                (_, Some(width)) => width.into(),
                (Direction::Vertical, None) => group
                    .elements
                    .iter()
                    .map(|e| e.width(state))
                    .try_fold(0, |acc, c| {
                        if let RatConstraint::Length(length) = c {
                            Some(length.max(acc))
                        } else {
                            None
                        }
                    })
                    .map(RatConstraint::Length)
                    .unwrap_or(RatConstraint::Percentage(100)),
                (Direction::Horizontal, None) => RatConstraint::Percentage(100),
            },
            Element::Text(text) => todo!(),
            Element::Bar(bar) => todo!(),
            Element::Graph(graph) => todo!(),
            Element::Image(image) => todo!(),
        }
    }
}
