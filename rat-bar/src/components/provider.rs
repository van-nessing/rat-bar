use std::{borrow::Cow, collections::HashMap, path::PathBuf, process::Stdio, time::Duration};

use color_eyre::{Section, SectionExt, eyre::Context};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use futures_concurrency::future::Race;
use itertools::Itertools;
use lazy_static::lazy_static;
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Position, Rect, Size},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};
use ratatui_image::protocol::Protocol;
use regex::Captures;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serde_with::{FromInto, serde_as};
use tokio::{io::AsyncReadExt, sync::mpsc::Sender};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
};

use crate::{
    app::State,
    event::{Event, Request},
    widgets::{
        bar_graph::{BarGraph, Marker},
        percentage_bar::BlockPercentageBar,
        scroll_text::{ScrollText, ScrollTextState},
    },
};

#[derive(Default)]
pub struct ProviderState {
    pub variables: HashMap<String, Value>,
    pub images: HashMap<String, AccessCache<Option<Protocol>>>,
}

pub struct AccessCache<T> {
    val: T,
    accessed: bool,
}

impl<T> AccessCache<T> {
    pub fn new(val: T) -> Self {
        Self {
            val,
            accessed: true,
        }
    }
    pub fn get(&mut self) -> &T {
        self.accessed = true;
        &self.val
    }
    pub fn reset(&mut self) {
        self.accessed = false;
    }
    pub fn accessed(&self) -> bool {
        self.accessed
    }
}

fn default_true() -> bool {
    true
}

fn default_flex() -> Flex {
    Flex::SpaceBetween
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub enum ProviderLayoutType {
    Interactable {
        inner: Box<ProviderLayoutType>,
        provider: String,
        on_click: Option<String>,
        on_scroll: Option<String>,
        on_drag: Option<String>,
    },
    HGroup {
        #[serde(default)]
        width: Constraint,
        #[serde(default = "default_flex")]
        flex: Flex,
        elements: Vec<ProviderLayoutType>,
    },
    VGroup {
        #[serde(default)]
        width: Option<Constraint>,
        #[serde(default = "default_true")]
        center: bool,
        elements: Vec<ProviderLayoutType>,
    },
    Text(#[serde_as(as = "FromInto<String>")] Text),
    Image {
        width: u16,
        var: String,
    },
    Bar {
        #[serde(default)]
        width: Constraint,
        direction: Direction,
        var: String,
        fg: String,
        bg: String,
    },
    Graph {
        #[serde(default)]
        width: Constraint,
        var: String,
        fg: String,
        fill: bool,
        marker: Marker,
    },
    Button {
        provider: String,
        name: String,
        text: String,
    },
}

pub struct Interactible {}

#[derive(Debug, Deserialize)]
pub enum InteractKind {
    Simple,
    Value { var: String },
}

#[derive(Debug)]
pub struct Text {
    string: String,
    state: ScrollTextState,
}

impl From<String> for Text {
    fn from(string: String) -> Self {
        Self {
            string,
            state: Default::default(),
        }
    }
}
#[derive(Serialize)]
pub struct ButtonMessage<'a> {
    name: &'a str,
    event: MouseEventKind,
}
#[derive(Serialize)]
pub struct Message<'a> {
    name: &'a str,
    value: f32,
}
#[derive(Serialize)]
pub struct BarMessage<'a> {
    message: &'a str,
    percentage: f32,
}

impl ProviderLayoutType {
    pub fn on_click(
        &self,
        provider: &str,
        message: &str,
        area: Rect,
        position: Position,
        dir: f32,
        state: &mut State,
    ) {
        if let ProviderLayoutType::Bar {
            width,
            direction,
            var,
            fg,
            bg,
            ..
        } = self
        {
            let percentage = state
                .providers
                .variables
                .get(var)
                .and_then(|var| var.as_f64())
                .unwrap_or(0.0) as f32;
            let new_percentage = match direction {
                Direction::Horizontal => (position.x - area.x) as f32 * 100.0 / area.width as f32,
                Direction::Vertical => {
                    (area.height - (position.y - area.y)) as f32 * 100.0 / area.height as f32
                }
            };
            let difference = new_percentage - percentage;
            let _ = state.requests.try_send(Request::MessageProvider {
                provider: provider.to_string(),
                message: serde_json::to_string(&json!({message: difference})).unwrap(),
            });
        } else {
            let _ = state.requests.try_send(Request::MessageProvider {
                provider: provider.to_string(),
                message: serde_json::to_string(&json!({message: dir})).unwrap(),
            });
        }
    }
    pub fn on_scroll(&self, provider: &str, message: &str, dir: f32, state: &mut State) {
        let _ = state.requests.try_send(Request::MessageProvider {
            provider: provider.to_string(),
            message: serde_json::to_string(&json!({message: dir})).unwrap(),
        });
    }
    pub fn width(&self, variables: &HashMap<String, Value>) -> Constraint {
        match self {
            ProviderLayoutType::Interactable { inner, .. } => inner.width(variables),
            ProviderLayoutType::HGroup { width, .. } => *width,
            ProviderLayoutType::VGroup {
                width, elements, ..
            } => {
                if let Some(width) = width {
                    *width
                } else {
                    elements
                        .iter()
                        .map(|e| e.width(variables))
                        .try_fold(0, |acc, c| {
                            if let Constraint::Length(len) = c {
                                Some(len.max(acc))
                            } else {
                                None
                            }
                        })
                        .map(Constraint::Length)
                        .unwrap_or(Constraint::Fill(1))
                }
            }
            ProviderLayoutType::Text(text) => {
                let string = interpolate(&text.string, variables);
                let line = format_string(string.as_ref());

                Constraint::Length(line.width() as u16)
            }
            ProviderLayoutType::Image { width, .. } => Constraint::Length(*width),
            ProviderLayoutType::Bar { width, .. } => *width,
            ProviderLayoutType::Graph { width, .. } => *width,
            ProviderLayoutType::Button {
                provider,
                name: message,
                text,
            } => {
                let string = interpolate(text, variables);
                let line = format_string(string.as_ref());

                Constraint::Length(line.width() as u16)
            }
        }
    }
    pub fn height(&self) -> Constraint {
        match self {
            ProviderLayoutType::Interactable { inner, .. } => inner.height(),
            ProviderLayoutType::HGroup { .. } => Constraint::Fill(1),
            ProviderLayoutType::VGroup { .. } => Constraint::Fill(1),
            ProviderLayoutType::Text(..) => Constraint::Length(1),
            ProviderLayoutType::Image { .. } => Constraint::Fill(1),
            ProviderLayoutType::Bar { direction, .. } => match direction {
                Direction::Horizontal => Constraint::Length(1),
                Direction::Vertical => Constraint::Fill(1),
            },
            ProviderLayoutType::Graph { .. } => Constraint::Fill(1),
            ProviderLayoutType::Button { .. } => Constraint::Length(1),
        }
    }
}

impl StatefulWidget for &mut ProviderLayoutType {
    type State = State;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) where
        Self: Sized,
    {
        match self {
            ProviderLayoutType::Interactable {
                inner,
                provider,
                on_click,
                on_scroll,
                on_drag,
            } => {
                let (click, scroll) = state
                    .mouse
                    .capture_events(area)
                    .into_iter()
                    .partition::<Vec<_>, _>(|e| e.kind.is_down());
                if let Some(message) = on_click {
                    for event in click {
                        let dir = match event.kind {
                            MouseEventKind::Down(MouseButton::Left) => 1.0,
                            MouseEventKind::Down(MouseButton::Right) => -1.0,
                            _ => continue,
                        };
                        inner.on_click(
                            provider,
                            message,
                            area,
                            Position::new(event.column, event.row),
                            dir,
                            state,
                        );
                    }
                }
                if let Some(message) = on_scroll {
                    for event in scroll {
                        let dir = match event.kind {
                            MouseEventKind::ScrollDown => -1.0,
                            MouseEventKind::ScrollUp => 1.0,
                            _ => continue,
                        };
                        inner.on_scroll(provider, message, dir, state);
                    }
                }

                inner.render(area, buf, state);
            }
            ProviderLayoutType::HGroup { flex, elements, .. } => {
                let constraints = elements
                    .iter()
                    .map(|element| element.width(&state.providers.variables));
                let layout =
                    area.layout_vec(&Layout::horizontal(constraints).spacing(1).flex(*flex));
                for (area, element) in layout.into_iter().zip(elements.iter_mut()) {
                    element.render(area, buf, state);
                }
            }
            ProviderLayoutType::VGroup {
                center, elements, ..
            } => {
                let constraints = elements.iter().map(ProviderLayoutType::height);
                let layout = area.layout_vec(&Layout::vertical(constraints));
                for (mut area, element) in layout.into_iter().zip(elements.iter_mut()) {
                    if *center {
                        area =
                            area.centered_horizontally(element.width(&state.providers.variables));
                    }
                    element.render(area, buf, state);
                }
            }
            ProviderLayoutType::Text(text) => {
                let string = interpolate(&text.string, &state.providers.variables);
                let line = format_string(string.as_ref());
                ScrollText { line }.render(area, buf, &mut text.state);
            }
            ProviderLayoutType::Image { var, .. } => {
                if let Some(path) = &state.providers.variables.get(var) {
                    let path = path.as_str().unwrap();
                    // image is present
                    if let Some(access) = state.providers.images.get_mut(path) {
                        // image finished loading
                        if let Some(protocol) = access.get() {
                            ratatui_image::Image::new(protocol).render(area, buf);
                        }
                    } else {
                        let _ = state.requests.try_send(Request::LoadImage {
                            path: path.to_string(),
                            size: Size::new(5, area.height),
                        });
                    }
                }
            }
            ProviderLayoutType::Bar {
                direction,
                var,
                fg,
                bg,
                ..
            } => {
                let percentage = state
                    .providers
                    .variables
                    .get(var)
                    .and_then(|var| var.as_f64())
                    .unwrap_or(0.0) as f32;
                let fg = interpolate(fg, &state.providers.variables);
                let bg = interpolate(bg, &state.providers.variables);

                let fg = get_color(&fg).unwrap_or(Color::White);
                let bg = get_color(&bg).unwrap_or(Color::DarkGray);

                let style = Style::new().fg(fg).bg(bg);

                BlockPercentageBar {
                    style,
                    percentage,
                    direction: *direction,
                }
                .render(area, buf);
            }
            ProviderLayoutType::Graph {
                var,
                fg,
                fill,
                marker,
                ..
            } => {
                if let Some(data) = state
                    .providers
                    .variables
                    .get(var)
                    .and_then(|var| var.as_array())
                    .and_then(|val| {
                        val.iter()
                            .map(|val| val.as_f64().map(|val| val as f32))
                            .collect::<Option<Vec<_>>>()
                    })
                {
                    let fg = interpolate(fg, &state.providers.variables);
                    let color = get_color(&fg).unwrap_or(Color::White);

                    BarGraph {
                        percentages: data.as_slice(),
                        datapoint_count: data.len(),
                        color,
                        fill: *fill,
                        marker: *marker,
                    }
                    .render(area, buf);
                }
            }
            ProviderLayoutType::Button {
                provider,
                name,
                text,
            } => {
                let captured = state.mouse.capture_events(area);

                for event in captured {
                    state
                        .requests
                        .try_send(Request::MessageProvider {
                            provider: provider.clone(),
                            message: serde_json::to_string(&ButtonMessage {
                                name,
                                event: event.kind,
                            })
                            .unwrap(),
                        })
                        .unwrap()
                }

                let string = interpolate(text, &state.providers.variables);
                let line = format_string(string.as_ref());
                line.render(area, buf);
            }
        }
    }
}

lazy_static! {
    static ref VARIABLES: regex::Regex = regex::Regex::new(r"\$\{(?<var>[^${}]*)\}").unwrap();
    static ref FORMAT: regex::Regex =
        regex::Regex::new(r"\$\[(?<args>[^\[\]]*)\]\((?<text>[^)\\]*(?:\\.[^)\\]*)*)\)").unwrap();
}

pub fn interpolate<'a>(string: &'a str, providers: &'_ HashMap<String, Value>) -> Cow<'a, str> {
    VARIABLES.replace_all(string, |captures: &Captures| {
        let var = captures.name("var").unwrap();
        providers
            .get(var.as_str())
            .map(|var| {
                if let Value::String(string) = &var {
                    Cow::Borrowed(string.as_str())
                } else {
                    Cow::Owned(var.to_string())
                }
            })
            .unwrap_or(Cow::Borrowed("UNDEFINED"))
    })
}
pub fn get_color(str: &str) -> Option<Color> {
    Some(match str {
        "Black" => Color::Black,
        "Red" => Color::Red,
        "Green" => Color::Green,
        "Yellow" => Color::Yellow,
        "Blue" => Color::Blue,
        "Magenta" => Color::Magenta,
        "Cyan" => Color::Cyan,
        "Gray" => Color::Gray,
        "DarkGray" => Color::DarkGray,
        "LightRed" => Color::LightRed,
        "LightGreen" => Color::LightGreen,
        "LightYellow" => Color::LightYellow,
        "LightBlue" => Color::LightBlue,
        "LightMagenta" => Color::LightMagenta,
        "LightCyan" => Color::LightCyan,
        "White" => Color::White,
        str if str.starts_with('#') => Color::from_u32(u32::from_str_radix(&str[1..], 16).ok()?),
        _ => return None,
    })
}

pub fn get_style(str: &str) -> Style {
    let styles = str.split(',');

    styles.fold(Style::default(), |style, str| match str.split_once(':') {
        Some((str, args)) => match str {
            "bg" => get_color(args).map(|c| style.bg(c)).unwrap_or(style),
            "fg" => get_color(args).map(|c| style.fg(c)).unwrap_or(style),
            _ => style,
        },
        None => match str {
            "ul" => style.underlined(),
            "rv" => style.reversed(),
            "it" => style.italic(),
            "bo" => style.bold(),
            "sb" => style.slow_blink(),
            "rb" => style.rapid_blink(),
            "cr" => style.crossed_out(),
            _ => style,
        },
    })
}

pub fn format_string<'a>(string: &'a str) -> Line<'a> {
    let mut start = 0;
    let mut line = Line::default();
    for captures in FORMAT.captures_iter(string) {
        let match_start = captures.get_match().start();
        let style = captures.name("args").unwrap();
        let text = captures.name("text").unwrap();
        let span = Span::from(text.as_str().replace(r"\)", ")").replace(r"\(", "("))
            .style(get_style(style.as_str()));

        if match_start > start {
            line.push_span(
                string[start..match_start]
                    .replace(r"\)", ")")
                    .replace(r"\(", "("),
            );
        }
        line.push_span(span);

        start = captures.get_match().end()
    }
    if start < string.len() {
        line.push_span(
            string[start..string.len()]
                .replace(r"\)", ")")
                .replace(r"\(", "("),
        );
    }

    line
}

fn expand_home(path: &str) -> color_eyre::Result<PathBuf> {
    if path == "~" {
        return Ok(PathBuf::from(std::env::var("HOME")?));
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return Ok(PathBuf::from(std::env::var("HOME")?).join(rest));
    }
    Ok(PathBuf::from(path))
}
pub async fn init_providers(
    providers: HashMap<String, crate::config::Provider>,
) -> color_eyre::Result<HashMap<String, Child>> {
    providers
        .into_iter()
        .map(|(name, config)| {
            let (program, args) = config
                .command
                .split_first()
                .ok_or_else(|| color_eyre::eyre::eyre!("provider program missing"))?;
            let path = expand_home(program)?;
            let mut command = tokio::process::Command::new(&path);
            command
                .args(args)
                .kill_on_drop(true)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map(|child| (name.clone(), child))
                .map_err(color_eyre::Report::from)
                .with_section(move || name.header("provider"))
                .with_section(move || path.to_string_lossy().to_string().header("command"))
                .with_section(move || args.iter().join(" ").header("arguments"))
        })
        .collect()
}

pub async fn provider_events(
    sender: Sender<Event>,
    providers: HashMap<String, Child>,
) -> color_eyre::Result<()> {
    providers
        .into_iter()
        .map(|(provider, mut child)| {
            let sender = sender.clone();
            async move {
                let result = async || {
                    let mut buf = String::new();
                    let mut stdout = child.stdout.take().unwrap();
                    let mut stderr = child.stderr.take().unwrap();
                    let mut reader = BufReader::new(&mut stdout);
                    loop {
                        buf.clear();
                        reader.read_line(&mut buf).await?;

                        let variables = match serde_json::from_str(&buf) {
                            Ok(var) => var,
                            Err(e) => {
                                let mut err = String::new();
                                tokio::time::timeout(
                                    Duration::from_secs(1),
                                    stderr.read_to_string(&mut err),
                                )
                                .await;
                                let err = color_eyre::Result::<()>::Err(e.into())
                                    .suppress_backtrace(true)
                                    .with_section(|| provider.header("provider"))
                                    .with_section(|| buf.header("stdout"))
                                    .with_section(|| err.header("stderr"));
                                return err;
                            }
                        };
                        sender
                            .send(Event::UpdateProvider {
                                name: provider.clone(),
                                variables,
                            })
                            .await?;
                    }
                };
                let result = result().await;
                let _ = child.kill().await;
                result
            }
        })
        .collect::<Vec<_>>()
        .race()
        .await
}
