use std::ops::{Deref, DerefMut};
use std::{num::ParseIntError, str::FromStr};

use crossterm::event::{MouseButton, MouseEventKind};
use knuffel::errors::DecodeError;
use knuffel::traits::ErrorSpan;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Stylize};
use ratatui::widgets::Widget;
use ratatui::widgets::{Block as RatBlock, Borders as RatBorders};
use ratatui::{layout::Constraint as RatConstraint, widgets::StatefulWidget};
use serde_json::json;

use crate::event::Request;
use crate::layout::style::interpolate_string;
use crate::layout::style::style_string;
use crate::{
    app::State,
    layout::{bar::Bar, graph::Graph, group::Group, image::Image, text::Text},
};

pub mod bar;
pub mod graph;
pub mod group;
pub mod image;
pub mod style;
pub mod text;

pub trait ElementWidget {
    fn width(&self, state: &State) -> RatConstraint;
    fn height(&self) -> RatConstraint;
    fn on_click(
        &self,
        ProviderMessage { provider, message }: ProviderMessage,
        _area: Rect,
        _click_position: Position,
        dir: f32,
        state: &mut State,
    ) {
        let _ = state.requests.try_send(Request::MessageProvider {
            provider,
            message: serde_json::to_string(&json!({message: dir})).unwrap(),
        });
    }
    fn on_scroll(
        &self,
        ProviderMessage { provider, message }: ProviderMessage,
        dir: f32,
        state: &mut State,
    ) {
        let _ = state.requests.try_send(Request::MessageProvider {
            provider,
            message: serde_json::to_string(&json!({message: dir})).unwrap(),
        });
    }
    fn interact(
        &self,
        on_click: Option<ProviderMessage>,
        on_scroll: Option<ProviderMessage>,
        area: Rect,
        state: &mut State,
    ) {
        let (clicks, scrolls) = state
            .mouse
            .capture_events(area)
            .into_iter()
            .partition::<Vec<_>, _>(|e| e.kind.is_down());
        if let Some(on_click) = on_click {
            for click in clicks {
                let position = Position::new(click.column, click.row);
                let dir = match click.kind {
                    MouseEventKind::Down(MouseButton::Left) => 1.0,
                    MouseEventKind::Down(MouseButton::Right) => -1.0,
                    _ => unreachable!(),
                };
                self.on_click(on_click.clone(), area, position, dir, state);
            }
        }
        if let Some(on_scroll) = on_scroll {
            for scroll in scrolls {
                let dir = match scroll.kind {
                    MouseEventKind::ScrollDown => -1.0,
                    MouseEventKind::ScrollUp => 1.0,
                    MouseEventKind::ScrollRight => 1.0,
                    MouseEventKind::ScrollLeft => -1.0,
                    _ => unreachable!(),
                };
                self.on_scroll(on_scroll.clone(), dir, state);
            }
        }
    }
    // fn interact(&self, )
}

#[derive(knuffel::Decode)]
pub struct BarElement {
    #[knuffel(child)]
    block: Option<Block>,
    #[knuffel(property)]
    fg: Option<String>,
    #[knuffel(property)]
    bg: Option<String>,
    #[knuffel(property, str, default)]
    pub width: Constraint,
    #[knuffel(children)]
    layout: Vec<Layout>,
}

pub struct Layout {
    n: u8,
    element: Element,
}

#[derive(knuffel::Decode)]
pub struct Block {
    #[knuffel(property)]
    title: Option<String>,
    #[knuffel(property)]
    fg: Option<String>,
    #[knuffel(property)]
    bg: Option<String>,
    #[knuffel(child)]
    borders: Option<Borders>,
}
#[derive(knuffel::Decode, Clone, Copy)]
pub struct Borders {
    #[knuffel(child)]
    left: bool,
    #[knuffel(child)]
    right: bool,
    #[knuffel(child)]
    top: bool,
    #[knuffel(child)]
    bottom: bool,
}
#[derive(knuffel::Decode)]
pub enum Element {
    Group(Group),
    Text(Text),
    Bar(Bar),
    Graph(Graph),
    Image(Image),
}

#[derive(Clone, Copy)]
pub enum Constraint {
    Length(u16),
    Percent(u16),
    Fill(u16),
}

#[derive(Clone, Copy)]
pub enum Direction {
    Horizontal,
    Vertical,
}
#[derive(Clone)]
pub struct ProviderMessage {
    provider: String,
    message: String,
}
impl From<Borders> for RatBorders {
    fn from(value: Borders) -> Self {
        let mut borders = RatBorders::empty();
        if value.top {
            borders |= RatBorders::TOP;
        }
        if value.bottom {
            borders |= RatBorders::BOTTOM;
        }
        if value.left {
            borders |= RatBorders::LEFT;
        }
        if value.right {
            borders |= RatBorders::RIGHT;
        }
        borders
    }
}
impl FromStr for ProviderMessage {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (provider, message) = s
            .split_once(".")
            .ok_or(r#"expected pattern: `provider.message`"#)?;
        Ok(ProviderMessage {
            provider: provider.to_string(),
            message: message.to_string(),
        })
    }
}

impl ElementWidget for Element {
    fn width(&self, state: &State) -> RatConstraint {
        match self {
            Element::Group(group) => group.width(state),
            Element::Text(text) => text.width(state),
            Element::Bar(bar) => bar.width(state),
            Element::Graph(graph) => graph.width(state),
            Element::Image(image) => image.width(state),
        }
    }

    fn height(&self) -> RatConstraint {
        match self {
            Element::Group(group) => group.height(),
            Element::Text(text) => text.height(),
            Element::Bar(bar) => bar.height(),
            Element::Graph(graph) => graph.height(),
            Element::Image(image) => image.height(),
        }
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
            Element::Group(group) => group.render(area, buf, state),
            Element::Text(text) => text.render(area, buf, state),
            Element::Bar(bar) => bar.render(area, buf, state),
            Element::Graph(graph) => graph.render(area, buf, state),
            Element::Image(image) => image.render(area, buf, state),
        }
    }
}

impl StatefulWidget for &mut BarElement {
    type State = State;

    fn render(
        self,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if let Some(block) = &self.block {
            let mut rat_block = RatBlock::default();

            if let Some(title) = &block.title {
                let string = interpolate_string(title, state);
                let line = style_string(string.into());
                rat_block = rat_block.title(line);
            }
            if let Some(fg) = &block.fg {
                rat_block = rat_block.fg(Color::from_str(fg).unwrap_or_default())
            }
            if let Some(bg) = &block.bg {
                rat_block = rat_block.bg(Color::from_str(bg).unwrap_or_default())
            }
            if let Some(borders) = block.borders {
                rat_block = rat_block.borders(borders.into());
            } else {
                rat_block = rat_block.borders(RatBorders::all());
            }

            (&rat_block).render(area, buf);
            area = rat_block.inner(area);
        }
        match area.height {
            0 => {}
            n if !self.layout.is_empty() => {
                let i = (n as usize).min(self.layout.len()) - 1;
                let element = &mut self.layout[i];
                element.render(area, buf, state);
            }
            _ => {}
        }
    }
}
impl FromStr for Direction {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "h" | "horizontal" => Ok(Self::Horizontal),
            "v" | "vertical" => Ok(Self::Vertical),
            _ => Err("expected one of: `v`, `vertical`, `h`, `horizontal`"),
        }
    }
}
impl From<Direction> for ratatui::layout::Direction {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Horizontal => Self::Horizontal,
            Direction::Vertical => Self::Vertical,
        }
    }
}

impl Default for Constraint {
    fn default() -> Self {
        Constraint::Fill(1)
    }
}
impl From<Constraint> for RatConstraint {
    fn from(value: Constraint) -> Self {
        match value {
            Constraint::Length(length) => RatConstraint::Length(length),
            Constraint::Percent(percent) => RatConstraint::Percentage(percent),
            Constraint::Fill(length) => RatConstraint::Fill(length),
        }
    }
}

impl FromStr for Constraint {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Some(s) = s.strip_suffix("%") {
            Self::Percent(s.parse()?)
        } else if let Some(s) = s.strip_suffix("#") {
            Self::Fill(s.parse()?)
        } else {
            Self::Length(s.parse()?)
        })
    }
}

impl<S: ErrorSpan> knuffel::Decode<S> for Layout {
    fn decode_node(
        node: &knuffel::ast::SpannedNode<S>,
        ctx: &mut knuffel::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        if let Some(children) = &node.children {
            if children.len() != 1 {
                Err(DecodeError::unexpected(
                    children,
                    "children",
                    format!("expected one child, found {}", children.len()),
                ))
            } else {
                let node = children.first().unwrap();
                Element::decode_node(node, ctx).map(|element| Layout { element, n: 0 })
            }
        } else {
            Err(DecodeError::missing(
                node,
                format!("missing child element, one of: text, group, etc..."),
            ))
        }
    }
}
impl DerefMut for Layout {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.element
    }
}
impl Deref for Layout {
    type Target = Element;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}
