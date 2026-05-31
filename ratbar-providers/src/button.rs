use serde::{Deserialize, Serialize};

use crate::Provider;
use std::{collections::BTreeMap, time::Duration};

pub struct Button {
    i: f32,
}

#[derive(clap::Args)]
pub struct ButtonArgs {}

#[derive(Serialize)]
pub struct ButtonFormat {
    i: f32,
}

#[derive(Deserialize)]
pub struct Input {
    click: Option<f32>,
}

impl Provider for Button {
    type Args = ButtonArgs;
    type Fmt<'a> = ButtonFormat;
    fn init(args: Self::Args) -> color_eyre::Result<Button> {
        Ok(Button { i: 0.0 })
    }
    fn duration(&self) -> Option<Duration> {
        None
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        Ok(ButtonFormat { i: self.i })
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        let input: Input = serde_json::from_str(&buf)?;
        if let Some(click) = input.click {
            self.i += click
        }

        Ok(())
    }
}
