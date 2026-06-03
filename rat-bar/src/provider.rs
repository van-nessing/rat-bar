use std::{collections::HashMap, path::PathBuf, process::Stdio, time::Duration};

use futures_concurrency::future::Race;
use itertools::Itertools;
use miette::{IntoDiagnostic, SourceOffset};
use ratatui_image::protocol::Protocol;
use serde_json::Value;
use tokio::{io::AsyncReadExt, sync::mpsc::Sender};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
};

use crate::event::Event;

#[derive(Default)]
pub struct ProviderState {
    pub variables: HashMap<String, Value>,
    pub images: HashMap<String, AccessCache<Option<Protocol>>>,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("malformed json provided")]
struct SerdeError {
    cause: serde_json::Error,
    #[source_code]
    input: String,
    #[label("{cause}")]
    location: SourceOffset,
}

impl SerdeError {
    /// Takes the input and the `serde_json::Error` and returns a SerdeError
    /// that can be rendered nicely with miette.
    pub fn from_serde_error(input: impl Into<String>, cause: serde_json::Error) -> Self {
        let input = input.into();
        let location = SourceOffset::from_location(&input, cause.line(), cause.column());
        Self {
            cause,
            input,
            location,
        }
    }
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

fn expand_home(path: &str) -> miette::Result<PathBuf> {
    if path == "~" {
        return Ok(PathBuf::from(std::env::var("HOME").into_diagnostic()?));
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return Ok(PathBuf::from(std::env::var("HOME").into_diagnostic()?).join(rest));
    }
    Ok(PathBuf::from(path))
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("could not spawn provider: `{provider}`")]
struct ProviderError {
    #[source]
    kind: ProviderErrorKind,
    provider: String,
}
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ProviderErrorKind {
    #[error("no program provided")]
    ProgramMissing,
    #[error("program `{0}` does not exist")]
    ProgramNotFound(String),

    #[error("program: `{program}`\nargs: `{args:?}`")]
    SpawnError {
        program: String,
        args: Vec<String>,
        source: tokio::io::Error,
    },
}
pub async fn init_providers(
    providers: HashMap<String, crate::config::Provider>,
) -> miette::Result<HashMap<String, Child>> {
    providers
        .into_iter()
        .map(|(name, config)| {
            let (program, args) = config.command.split_first().ok_or_else(|| ProviderError {
                kind: ProviderErrorKind::ProgramMissing,
                provider: name.clone(),
            })?;
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
                .map_err(|e| ProviderError {
                    provider: name.clone(),
                    kind: ProviderErrorKind::SpawnError {
                        program: path.to_string_lossy().to_string(),
                        args: Vec::from(args),
                        source: e,
                    },
                })
                .map_err(miette::Report::from)
        })
        .collect()
}

pub async fn provider_events(
    sender: Sender<Event>,
    providers: HashMap<String, Child>,
) -> miette::Result<()> {
    providers
        .into_iter()
        .map(|(provider, mut child)| {
            let sender = sender.clone();
            async move {
                let mut result = async || {
                    let mut buf = String::new();
                    let mut stdout = child.stdout.take().unwrap();
                    let mut stderr = child.stderr.take().unwrap();
                    let mut reader = BufReader::new(&mut stdout);
                    loop {
                        buf.clear();
                        reader.read_line(&mut buf).await.into_diagnostic()?;

                        let variables = match serde_json::from_str(&buf) {
                            Ok(var) => var,
                            Err(e) => {
                                let mut err = String::new();
                                tokio::time::timeout(
                                    Duration::from_secs(1),
                                    stderr.read_to_string(&mut err),
                                )
                                .await;
                                Err(SerdeError::from_serde_error(&buf, e))?
                                // let err = color_eyre::Result::<()>::Err(e.into())
                                //     .suppress_backtrace(true)
                                //     .with_section(|| provider.header("provider"))
                                //     .with_section(|| buf.header("stdout"))
                                //     .with_section(|| err.header("stderr"));
                                // return err;
                            }
                        };
                        sender
                            .send(Event::UpdateProvider {
                                name: provider.clone(),
                                variables,
                            })
                            .await
                            .into_diagnostic()?;
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
