use crate::Config;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, path::PathBuf};

pub struct ConfigLoader<T> {
    file: FileSpec,
    _p: PhantomData<T>,
}

impl<T> ConfigLoader<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    pub(crate) fn new(file: FileSpec) -> Self {
        Self {
            file,
            _p: PhantomData::default(),
        }
    }

    pub fn load(&self) -> Result<Config<T>> {
        let value: T = match &self.file {
            FileSpec::Json(p) => serde_json::from_reader(std::fs::File::open(p)?)?,
            FileSpec::Yaml(p) => serde_yaml::from_reader(std::fs::File::open(p)?)?,
        };
        Config::new_with(value)
    }
}

pub(crate) enum FileSpec {
    Json(PathBuf),
    Yaml(PathBuf),
}
