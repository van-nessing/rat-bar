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
    #[serde_as(as = "Option<DurationSecondsWithFrac<f64>>")]
    pub update: Option<Duration>,
    pub command: Vec<String>,
}
