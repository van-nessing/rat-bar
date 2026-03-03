use std::{collections::HashMap, time::Duration};

use serde::Deserialize;
use serde_with::{DurationSecondsWithFrac, serde_as};
use tokio::process::Child;

#[derive(Deserialize)]
pub struct Config {
    pub providers: HashMap<String, String>,
}

#[serde_as]
#[derive(Deserialize)]
pub struct Provider {
    pub command: Vec<String>,
}
