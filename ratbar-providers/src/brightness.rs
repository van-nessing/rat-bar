use serde::Deserialize;

use crate::Provider;
use std::{fs::read_to_string, io::BufRead, sync::mpsc::Receiver, time::Duration};

pub struct Brightness {
    args: BrightnessArgs,
    events: Receiver<Event>,
    max: u64,
    now: u64,
}

enum Event {
    Tick,
    Message(Message),
}

#[derive(Debug, Deserialize)]
struct Message {
    add_brightness: Option<f32>,
}

#[derive(clap::Args)]
pub struct BrightnessArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    /// Display name
    display: String,
    #[arg(long, default_value_t = 10)]
    min_percent: u8,
}

#[derive(serde::Serialize)]
pub struct BrightnessFormat {
    brighness: u8,
}

impl Provider for Brightness {
    type Args = BrightnessArgs;
    type Fmt<'a> = BrightnessFormat;
    fn init(args: Self::Args) -> color_eyre::Result<Brightness> {
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        {
            let tx = tx.clone();
            std::thread::spawn(move || -> color_eyre::Result<()> {
                let stdin = std::io::stdin();
                let mut reader = std::io::BufReader::new(stdin);
                let mut buf = String::new();
                loop {
                    buf.clear();
                    reader.read_line(&mut buf)?;
                    let message = serde_json::from_str::<Message>(&buf)?;
                    tx.send(Event::Message(message))?;
                }
            });
        }
        {
            let tx = tx.clone();
            std::thread::spawn(move || -> color_eyre::Result<()> {
                loop {
                    tx.send(Event::Tick)?;
                    std::thread::sleep(args.duration);
                }
            });
        }
        Ok(Brightness {
            args,
            events: rx,
            max: 100,
            now: 100,
        })
    }
    fn duration(&self) -> Option<Duration> {
        None
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        let event = self.events.recv()?;
        match event {
            Event::Tick => {
                let now = read_to_string(format!(
                    "/sys/class/backlight/{}/brightness",
                    self.args.display
                ))?;
                let max = read_to_string(format!(
                    "/sys/class/backlight/{}/max_brightness",
                    self.args.display
                ))?;

                self.now = now.trim().parse()?;
                self.max = max.trim().parse()?;
            }
            Event::Message(message) => {
                if let Some(percentage) = message.add_brightness {
                    if percentage.is_sign_positive() {
                        self.now += percentage.abs() as u64 * self.max;
                    } else {
                        self.now -= percentage.abs() as u64 * self.max;
                    }
                }
                let display = self.args.display.clone();
                let now = self.now;
                std::thread::spawn(move || {
                    std::fs::write(format!("/sys/class/backlight/{}", display), now.to_string())
                });
            }
        }

        Ok(())
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        Ok(BrightnessFormat {
            brighness: (self.now * 100 / self.max) as u8,
        })
    }
}
