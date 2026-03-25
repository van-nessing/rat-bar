use std::{io::Read, os::fd::IntoRawFd};

use clap::Parser;
use color_eyre::{self as eyre, Section, SectionExt, eyre::Context};
use eyre::eyre::eyre;
use futures_concurrency::{
    future::Race as _,
    prelude::{ConcurrentStream, IntoConcurrentStream},
};
use serde::Deserialize;
use smol::channel;
use tempfile::{NamedTempFile, tempfile};

#[derive(Parser)]
pub enum Commands {
    Spawn {
        screens: Option<Vec<String>>,
        #[arg(long, short, default_value_t = 4)]
        lines: u8,
        #[arg(long, short, default_value = "rat-bar")]
        bin: String,
    },
    Resize {
        lines: u8,
        screens: Option<Vec<String>>,
    },
}

#[derive(Deserialize)]
pub struct Screen {
    description: String,
    name: String,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let commands = Commands::parse();
    smol::block_on(async {
        match commands {
            Commands::Spawn {
                screens,
                lines,
                bin,
            } => {
                let mut screens = if let Some(screens) = screens {
                    screens
                } else {
                    get_screens().await?
                };
                let temp_file = NamedTempFile::new()?;
                let temp_path = temp_file.path().to_string_lossy();

                if let Some(first) = screens.pop() {
                    let (tx, rx) = channel::bounded::<()>(1);

                    ctrlc::set_handler(move || {
                        let _ = tx.send_blocking(());
                    })?;

                    let mut first =
                        spawn_on(first, lines, &["sh", "-c", &format!("{bin} 2>{temp_path}")])
                            .spawn()?;
                    let rest = screens
                        .into_co_stream()
                        .map(async |screen| spawn_on(screen, lines, &[bin.as_str()]).spawn())
                        .collect::<Result<Vec<_>, _>>()
                        .await;

                    let out = (async { Some(first.status().await) }, async {
                        let _ = rx.recv().await;
                        None
                    })
                        .race()
                        .await;

                    first.kill()?;
                    if let Ok(rest) = rest {
                        for mut child in rest {
                            let _ = child.kill();
                        }
                    }
                    if let Some(code) = out {
                        let mut err = eyre!("failed to launch rat-bar");
                        return Err(err.with_section(|| {
                            let mut buf = String::new();
                            let _ = temp_file.into_file().read_to_string(&mut buf);
                            buf.header("rat-bar:")
                        }));
                    }
                }
            }
            Commands::Resize { lines, screens } => {
                let screens = if let Some(screen) = screens {
                    screen
                } else {
                    get_screens().await?
                };

                let results = screens
                    .into_co_stream()
                    .map(async |screen| {
                        smol::process::Command::new("kitten")
                            .args([
                                "@",
                                format!("--to=unix:/tmp/rat-bar-{screen}").as_str(),
                                "resize-os-window",
                                "--action=os-panel",
                                format!("lines={lines}").as_str(),
                            ])
                            .output()
                            .await
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .await?;
                for result in results {
                    if !result.status.success() {
                        let stderr = String::from_utf8_lossy(&result.stderr);
                        return Err(eyre!("failed to resize"))
                            .with_section(|| stderr.trim().to_string());
                    }
                }
            }
        }
        Ok(())
    })
}

async fn get_screens() -> eyre::Result<Vec<String>> {
    let out = smol::process::Command::new("kitten")
        .args(["panel", "--output-name=listjson"])
        .output()
        .await
        .wrap_err_with(|| eyre!("error while using kitten panel, is kitty installed?"))?;
    if !out.status.success() {
        return Err(eyre!("kitten panel returned unsuccessfully"))
            .wrap_err_with(|| eyre!("stderr: {}", String::from_utf8_lossy(&out.stderr)));
    }
    let screens: Vec<Screen> = serde_json::from_slice(&out.stdout)
        .wrap_err_with(|| eyre!("could not parse screens"))
        .wrap_err_with(|| eyre!("stdout: {}", String::from_utf8_lossy(&out.stdout)))?;

    Ok(screens.into_iter().map(|screen| screen.name).collect())
}

fn spawn_on(screen: String, lines: u8, bin: &[&str]) -> smol::process::Command {
    let mut command = smol::process::Command::new("kitten");
    command
        .args([
            "panel",
            "--edge=top",
            format!("--output-name={screen}").as_str(),
            format!("--lines={lines}").as_str(),
            format!("--listen-on=unix:/tmp/rat-bar-{screen}").as_str(),
            "-o",
            "window_padding_width=0",
            "-o",
            "allow_remote_control=yes",
        ])
        .args(bin);
    command
}
