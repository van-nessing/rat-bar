use std::{borrow::Cow, cell::Cell, collections::HashMap, process::Stdio, rc::Rc, time::Duration};

use color_eyre::eyre::Context;
use futures_concurrency::future::Race;
use lazy_static::lazy_static;
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Size},
    style::Style,
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};
use ratatui_image::protocol::Protocol;
use regex::Captures;
use serde::Deserialize;
use serde_json::Value;
use serde_with::{FromInto, serde_as};
use tokio::{io::AsyncReadExt, sync::mpsc::Sender};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Child,
};

use crate::{
    event::{Event, Request},
    widgets::{
        graph::GraphWidget,
        percentage_bar::BlockPercentageBar,
        scroll_text::{ScrollText, ScrollTextState},
    },
};

// #[derive(Debug)]
pub struct ProviderMeta {
    pub providers: HashMap<String, Provider>,
    pub images: HashMap<String, AccessBuf<Option<Protocol>>>,
}
impl std::fmt::Debug for ProviderMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct AccessBuf<T> {
    val: T,
    accessed: bool,
}

impl<T> AccessBuf<T> {
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

#[derive(Debug)]
pub struct Provider {
    pub variables: HashMap<String, Variable>,
}

#[derive(Debug)]
pub struct Variable {
    pub value: Value,
}

pub struct ProviderProcess {
    pub process: Child,
}

pub struct ProviderLayout<'a> {
    pub variables: &'a HashMap<String, Variable>,
    pub images: &'a mut HashMap<String, AccessBuf<Option<Protocol>>>,
    pub layout: &'a mut ProviderLayoutType,
    pub requests: &'a mut Sender<Request>,
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
    },
    Graph {
        #[serde(default)]
        width: Constraint,
        var: String,
    },
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

pub struct ProviderWidget<'a> {
    pub meta: &'a Provider,
    pub images: &'a mut HashMap<String, AccessBuf<Option<Protocol>>>,
    pub layout: &'a mut [ProviderLayoutType],
    pub requests: &'a mut Sender<Request>,
}

impl Provider {
    pub fn update(&mut self, other: HashMap<String, Value>) {
        self.variables = other
            .into_iter()
            .map(|(var, val)| (var, Variable { value: val }))
            .collect();
    }
}

impl ProviderLayoutType {
    pub fn width(&self, variables: &HashMap<String, Variable>) -> Constraint {
        match self {
            ProviderLayoutType::HGroup {
                width,
                flex,
                elements,
            } => *width,
            ProviderLayoutType::VGroup {
                width,
                center,
                elements,
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
            ProviderLayoutType::Image { width, var } => Constraint::Length(*width),
            ProviderLayoutType::Bar {
                width,
                direction,
                var,
            } => *width,
            ProviderLayoutType::Graph { width, var } => *width,
        }
    }
    pub fn height(&self) -> Constraint {
        match self {
            ProviderLayoutType::HGroup {
                width,
                flex,
                elements,
            } => Constraint::Fill(1),
            ProviderLayoutType::VGroup {
                width,
                center,
                elements,
            } => Constraint::Fill(1),
            ProviderLayoutType::Text(..) => Constraint::Length(1),
            ProviderLayoutType::Image { width, var } => Constraint::Fill(1),
            ProviderLayoutType::Bar {
                width,
                direction,
                var,
            } => match direction {
                Direction::Horizontal => Constraint::Length(1),
                Direction::Vertical => Constraint::Fill(1),
            },
            ProviderLayoutType::Graph { width, var } => Constraint::Fill(1),
        }
    }
}

impl Widget for ProviderLayout<'_> {
    fn render(mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match &mut self.layout {
            ProviderLayoutType::HGroup {
                width,
                flex,
                elements,
            } => {
                let constraints = elements.iter().map(|element| element.width(self.variables));
                let layout =
                    area.layout_vec(&Layout::horizontal(constraints).spacing(1).flex(*flex));
                for (area, element) in layout.into_iter().zip(elements.iter_mut()) {
                    ProviderLayout {
                        variables: self.variables,
                        images: self.images,
                        layout: element,
                        requests: self.requests,
                    }
                    .render(area, buf);
                }
            }
            ProviderLayoutType::VGroup {
                width,
                center,
                elements,
            } => {
                let constraints = elements.iter().map(ProviderLayoutType::height);
                let layout = area.layout_vec(&Layout::vertical(constraints));
                for (mut area, element) in layout.into_iter().zip(elements.iter_mut()) {
                    if *center {
                        area = area.centered_horizontally(element.width(self.variables));
                    }
                    ProviderLayout {
                        variables: self.variables,
                        images: self.images,
                        layout: element,
                        requests: self.requests,
                    }
                    .render(area, buf);
                }
            }
            ProviderLayoutType::Text(text) => {
                let string = interpolate(&text.string, self.variables);
                let line = format_string(string.as_ref());
                ScrollText { line }.render(area, buf, &mut text.state);
            }
            ProviderLayoutType::Image { width, var } => {
                if let Some(path) = self.variables.get(var) {
                    let path = path.value.as_str().unwrap();
                    // image is present
                    if let Some(access) = self.images.get_mut(path) {
                        // image finished loading
                        if let Some(protocol) = access.get() {
                            ratatui_image::Image::new(protocol).render(area, buf);
                        }
                    } else {
                        self.requests.try_send(Request::LoadImage {
                            path: path.to_string(),
                            size: Size::new(5, area.height),
                        });
                    }
                }
            }
            ProviderLayoutType::Bar {
                width,
                direction,
                var,
            } => {
                if let Some(percentage) = self.variables.get(var).and_then(|var| var.value.as_f64())
                {
                    BlockPercentageBar {
                        style: Style::new().on_dark_gray(),
                        percentage: percentage as f32,
                        direction: *direction,
                    }
                    .render(area, buf);
                }
            }
            ProviderLayoutType::Graph { width, var } => {
                if let Some(data) = self
                    .variables
                    .get(var)
                    .and_then(|var| var.value.as_array())
                    .and_then(|val| {
                        val.iter()
                            .map(|val| val.as_f64().map(|val| val as f32))
                            .collect::<Option<Vec<_>>>()
                    })
                {
                    GraphWidget {
                        percentages: data.as_slice(),
                        datapoint_count: data.len(),
                    }
                    .render(area, buf);
                }
            }
        }
    }
}

impl Widget for ProviderWidget<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        if area.height == 0 {
            return;
        }
        let layout = self.layout.get_mut(area.height as usize - 1);

        let layout = if let Some(layout) = layout {
            layout
        } else {
            self.layout.last_mut().unwrap()
        };
        ProviderLayout {
            variables: &self.meta.variables,
            images: self.images,
            layout,
            requests: self.requests,
        }
        .render(area, buf);
    }
}

lazy_static! {
    static ref VARIABLES: regex::Regex = regex::Regex::new(r"\$\{([^${}]*)\}").unwrap();
    static ref FORMAT: regex::Regex = regex::Regex::new(r"\$(\w{2})\(([^)]*)\)").unwrap();
}

pub fn interpolate<'a>(string: &'a str, variables: &'_ HashMap<String, Variable>) -> Cow<'a, str> {
    VARIABLES.replace_all(string, |captures: &Captures| {
        let name = captures.get(1).unwrap();
        variables
            .get(name.as_str())
            .map(|var| {
                if let Value::String(string) = &var.value {
                    Cow::Borrowed(string.as_str())
                } else {
                    Cow::Owned(var.value.to_string())
                }
            })
            .unwrap_or(Cow::Borrowed("UNDEFINED"))
    })
}

pub fn get_style(str: &str) -> Style {
    let style = Style::default();
    match str {
        "ul" => style.underlined(),
        _ => style,
    }
}

pub fn format_string<'a>(string: &'a str) -> Line<'a> {
    let mut start = 0;
    let mut line = Line::default();
    for captures in FORMAT.captures_iter(string) {
        let match_start = captures.get_match().start();
        let style = captures.get(1).unwrap();
        let text = captures.get(2).unwrap();
        let span = Span::from(text.as_str()).style(get_style(style.as_str()));

        if match_start > start {
            line.push_span(&string[start..match_start]);
        }
        line.push_span(span);

        start = captures.get_match().end()
    }
    if start < string.len() {
        line.push_span(&string[start..string.len()]);
    }

    line
}

pub async fn provider_events(
    sender: Sender<Event>,
    providers: HashMap<String, crate::config::Provider>,
) -> color_eyre::Result<()> {
    let mut providers = providers
        .into_iter()
        .map(|(name, config)| {
            let (program, args) = config
                .command
                .split_first()
                .ok_or_else(|| color_eyre::eyre::eyre!("provider program missing"))?;

            let mut command = tokio::process::Command::new(program);
            command
                .args(args)
                .kill_on_drop(true)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map(|child| (name.clone(), ProviderProcess { process: child }))
                .map_err(color_eyre::Report::from)
                .map_err(|e| e.wrap_err(format!("provider: {name}")))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;

    let futures = providers
        .into_iter()
        .map(|(name, mut provider)| {
            let sender = sender.clone();
            async move {
                let mut buf = String::new();
                let mut stdin = provider.process.stdin.as_mut().unwrap();
                let mut stdout = provider.process.stdout.as_mut().unwrap();
                let mut stderr = provider.process.stderr.as_mut().unwrap();
                let mut reader = BufReader::new(&mut stdout);

                let result = (async || {
                    loop {
                        buf.clear();
                        reader.read_line(&mut buf).await?;

                        let variables = match serde_json::from_str(&buf) {
                            Ok(var) => var,
                            Err(e) => {
                                let mut err = Vec::new();
                                let another = tokio::time::timeout(
                                    Duration::from_secs(1),
                                    stderr.read_to_end(&mut err),
                                )
                                .await
                                .ok()
                                .and_then(|ok| ok.err());
                                let err = color_eyre::Result::<()>::Err(e.into())
                                    .wrap_err(format!("on provider: {name}"))
                                    .wrap_err(format!("output: {buf}"))
                                    .wrap_err(String::from_utf8_lossy(&err).to_string());
                                if let Some(another) = another {
                                    return err.wrap_err(another);
                                } else {
                                    return err;
                                }
                            }
                        };
                        sender
                            .send(Event::UpdateProvider {
                                name: name.clone(),
                                variables,
                            })
                            .await?;
                    }
                })()
                .await;
                provider.process.kill().await;
                result
            }
        })
        .collect::<Vec<_>>();

    futures.race().await
}
