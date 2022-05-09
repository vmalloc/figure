use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use reqwest::Url;
use serde_json::Value;

#[derive(Clone)]
pub(crate) enum Layer {
    File(FileSpec),
    JsonUrl(Url),
}

impl Layer {
    pub(crate) fn json_file(path: impl Into<PathBuf>) -> Self {
        Self::File(FileSpec::Json(path.into()))
    }

    pub(crate) fn yaml_file(path: impl Into<PathBuf>) -> Self {
        Self::File(FileSpec::Yaml(path.into()))
    }

    pub(crate) fn load(&self) -> Result<Value> {
        Ok(match self {
            Self::File(spec) => {
                let path = spec.path();
                let file = std::fs::File::open(spec.path())?;
                match &spec {
                    FileSpec::Json(_) => serde_json::from_reader(file).map_err(anyhow::Error::from),
                    FileSpec::Yaml(_) => serde_yaml::from_reader(file).map_err(anyhow::Error::from),
                }
                .with_context(|| format!("Failed loading configuration overlay from {path:?}"))?
            }
            Self::JsonUrl(url) => {
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()?;
                client
                    .get(url.to_owned())
                    .send()
                    .and_then(|resp| resp.json())
                    .with_context(|| format!("Failed reading configuration from {url}"))?
            }
        })
    }
}

#[derive(Clone)]
pub(crate) enum FileSpec {
    Json(PathBuf),
    Yaml(PathBuf),
}

impl FileSpec {
    fn path(&self) -> &Path {
        match self {
            FileSpec::Json(p) => p,
            FileSpec::Yaml(p) => p,
        }
    }
}
