use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_with::serde_as;
use std::{collections::HashMap, ffi::OsStr, path::Path};

use crate::layout::BarElement;

pub struct Config {
    pub providers: HashMap<String, Provider>,
    pub layout: Vec<BarElement>,
}

impl Config {
    pub fn load(providers: Option<&Path>, layout: Option<&Path>) -> miette::Result<Self> {
        let dir = dirs::config_local_dir()
            .ok_or_else(|| miette::miette!("couldn't find config directory"))?
            .join("rat-bar");

        Ok(Config {
            providers: load_all(&dir, "providers", providers)?,
            layout: load_layout(&dir, "layout.kdl", layout)?,
        })
    }
}
fn load_layout(dir: &Path, file: &str, path: Option<&Path>) -> miette::Result<Vec<BarElement>> {
    let path = if let Some(path) = path {
        path.to_path_buf()
    } else {
        dir.join(file)
    };
    let string = std::fs::read_to_string(&path).into_diagnostic()?;
    Ok(knuffel::parse(
        path.file_name().and_then(OsStr::to_str).unwrap_or_default(),
        &string,
    )?)
}
fn load_all<T: DeserializeOwned>(dir: &Path, file: &str, path: Option<&Path>) -> miette::Result<T> {
    if let Some((path, ext)) = path.and_then(|path| {
        path.extension()
            .and_then(OsStr::to_str)
            .map(|ext| (path, ext))
    }) {
        match ext {
            "yaml" | "yml" => load_yaml(path),
            "kdl" => load_kdl(path),
            _ => Err(miette::miette!("invalid extension: {ext}")),
        }
    } else {
        let things: &[(&dyn Fn(&Path) -> _, _)] = &[
            (&load_kdl, "kdl"),
            (&load_yaml, "yaml"),
            (&load_yaml, "yml"),
        ];
        for (fun, ext) in things {
            if let Ok(out) = fun(&dir.join(file).with_extension(ext)) {
                return Ok(out);
            }
        }
        Err(miette::miette!(
            "could not find valid config file in: {}",
            dir.display()
        ))
    }
}

fn load_kdl<T: DeserializeOwned>(path: &Path) -> miette::Result<T> {
    let _input = std::fs::read_to_string(path).into_diagnostic()?;
    // Ok(kdl::de::from_str(&input)?)
    todo!()
}
fn load_yaml<T: DeserializeOwned>(path: &Path) -> miette::Result<T> {
    let slice = std::fs::read(path).into_diagnostic()?;
    let deserializer = serde_yaml::Deserializer::from_slice(&slice);
    Ok(serde_yaml::with::singleton_map_recursive::deserialize(deserializer).into_diagnostic()?)
}
#[serde_as]
#[derive(Deserialize, Serialize)]
pub struct Provider {
    pub command: Vec<String>,
}
