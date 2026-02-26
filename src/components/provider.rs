use std::{borrow::Cow, collections::HashMap, process::Stdio, time::Duration};

use color_eyre::eyre::Context;
use futures_concurrency::future::Race;
use lazy_static::lazy_static;
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout},
    style::Style,
    widgets::Widget,
};
use regex::Captures;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Child,
};

use crate::{
    event::Event,
    widgets::{graph::GraphWidget, percentage_bar::BlockPercentageBar},
};

#[derive(Debug)]
pub struct ProviderMeta {
    pub providers: HashMap<String, Provider>,
}

#[derive(Debug)]
pub struct Provider {
    pub variables: HashMap<String, Value>,
}

pub struct ProviderProcess {
    pub update: Option<Duration>,
    pub process: Child,
}

pub struct ProviderLayout<'a> {
    pub variables: &'a HashMap<String, Value>,
    pub layout: &'a ProviderLayoutType,
}
fn default_true() -> bool {
    true
}
fn default_flex() -> Flex {
    Flex::SpaceBetween
}
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
        width: Constraint,
        #[serde(default = "default_true")]
        inherit: bool,
        #[serde(default = "default_true")]
        center: bool,
        elements: Vec<ProviderLayoutType>,
    },
    Text(String),
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

pub struct ProviderWidget<'a> {
    pub meta: &'a Provider,
    pub layout: &'a [ProviderLayoutType],
}
impl ProviderLayoutType {
    pub fn width(&self, variables: &HashMap<String, Value>) -> Constraint {
        match self {
            ProviderLayoutType::HGroup {
                width,
                flex,
                elements,
            } => *width,
            ProviderLayoutType::VGroup {
                width,
                inherit,
                center,
                elements,
            } => {
                if *inherit {
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
                        .unwrap_or(*width)
                } else {
                    *width
                }
            }
            ProviderLayoutType::Text(text) => {
                Constraint::Length(interpolate(text, variables).chars().count() as u16)
            }
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
                inherit,
                center,
                elements,
            } => Constraint::Fill(1),
            ProviderLayoutType::Text(_) => Constraint::Length(1),
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
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match &self.layout {
            ProviderLayoutType::HGroup {
                width,
                flex,
                elements,
            } => {
                let constraints = elements.iter().map(|element| element.width(self.variables));
                let layout =
                    area.layout_vec(&Layout::horizontal(constraints).spacing(1).flex(*flex));
                for (area, element) in layout.into_iter().zip(elements.iter()) {
                    ProviderLayout {
                        variables: self.variables,
                        layout: element,
                    }
                    .render(area, buf);
                }
            }
            ProviderLayoutType::VGroup {
                width,
                inherit,
                center,
                elements,
            } => {
                let constraints = elements.iter().map(ProviderLayoutType::height);
                let layout = area.layout_vec(&Layout::vertical(constraints));
                for (mut area, element) in layout.into_iter().zip(elements.iter()) {
                    if *center {
                        area = area.centered_horizontally(element.width(self.variables));
                    }
                    ProviderLayout {
                        variables: self.variables,
                        layout: element,
                    }
                    .render(area, buf);
                }
            }
            ProviderLayoutType::Text(text) => {
                let text = interpolate(text, self.variables);
                text.render(area, buf);
            }
            ProviderLayoutType::Bar {
                width,
                direction,
                var,
            } => {
                if let Some(percentage) = self.variables.get(var).and_then(|val| val.as_f64()) {
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
                    .and_then(|val| val.as_array())
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
        let layout = self
            .layout
            .get(area.height as usize - 1)
            .unwrap_or_else(|| self.layout.last().unwrap());
        ProviderLayout {
            variables: &self.meta.variables,
            layout,
        }
        .render(area, buf);
    }
}
lazy_static! {
    static ref REGEX: regex::Regex = regex::Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();
}

pub fn interpolate<'a>(string: &'a str, variables: &'_ HashMap<String, Value>) -> Cow<'a, str> {
    REGEX.replace_all(string, |captures: &Captures| {
        let name = captures.get(1).unwrap();
        variables
            .get(name.as_str())
            .map(|val| {
                if let Value::String(string) = val {
                    Cow::Borrowed(string.as_str())
                } else {
                    Cow::Owned(val.to_string())
                }
            })
            .unwrap_or(Cow::Borrowed("UNDEFINED"))
    })
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
                .stderr(Stdio::null())
                .spawn()
                .map(|child| {
                    (
                        name.clone(),
                        ProviderProcess {
                            update: config.update,
                            process: child,
                        },
                    )
                })
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
                let mut reader = BufReader::new(&mut stdout);

                let result = (async || {
                    if let Some(tick_duration) = provider.update {
                        let mut timer = tokio::time::interval(tick_duration);
                        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                        loop {
                            stdin.write_all(b"\n").await?;
                            stdin.flush().await?;

                            buf.clear();
                            reader.read_line(&mut buf).await?;

                            let variables = serde_json::from_str(&buf)?;

                            sender
                                .send(Event::UpdateProvider {
                                    name: name.clone(),
                                    provider: Provider { variables },
                                })
                                .await?;
                            timer.tick().await;
                        }
                    } else {
                        loop {
                            buf.clear();
                            reader.read_line(&mut buf).await?;

                            let variables = serde_json::from_str(&buf)
                                .wrap_err_with(|| format!("{name} input: {buf}"))?;

                            sender
                                .send(Event::UpdateProvider {
                                    name: name.clone(),
                                    provider: Provider { variables },
                                })
                                .await?;
                        }
                    }
                })()
                .await;
                provider.process.kill().await?;
                result
            }
        })
        .collect::<Vec<_>>();

    futures.race().await
}
