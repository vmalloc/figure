use crate::Config;
use anyhow::{Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

#[cfg(test)]
const WATCH_INTERVAL: Duration = Duration::from_millis(1);
#[cfg(not(test))]
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct ConfigLoader<T>
where
    T: Send + Sync + 'static,
{
    files: Vec<FileSpec>,
    _p: PhantomData<T>,
}

impl<T> ConfigLoader<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
{
    pub(crate) fn new(file: FileSpec) -> Self {
        Self {
            files: vec![file],
            _p: PhantomData::default(),
        }
    }

    pub fn and_overlay_json(self, path: impl Into<PathBuf>) -> Self {
        self.and_overlay(FileSpec::Json(path.into()))
    }

    pub fn and_overlay_yaml(self, path: impl Into<PathBuf>) -> Self {
        self.and_overlay(FileSpec::Yaml(path.into()))
    }

    fn and_overlay(mut self, file: FileSpec) -> Self {
        self.files.push(file);
        self
    }

    pub fn load(&self) -> Result<Config<T>> {
        let returned = Config::new_with(self.load_value()?)?;

        Ok(returned)
    }

    fn load_value(&self) -> Result<T> {
        let mut value = serde_json::Value::Null;

        for overlay in &self.files {
            let overlay_value: serde_json::Value = match overlay {
                FileSpec::Json(p) => serde_json::from_reader(std::fs::File::open(p)?)
                    .with_context(|| {
                        format!("Failed loading configuration overlay from {:?}", p)
                    })?,
                FileSpec::Yaml(p) => serde_yaml::from_reader(std::fs::File::open(p)?)
                    .with_context(|| {
                        format!("Failed loading configuration overlay from {:?}", p)
                    })?,
            };
            json_patch::merge(&mut value, &overlay_value);
        }

        let inner =
            serde_json::from_value(value).context("Failed deserializing value from files")?;
        Ok(inner)
    }

    pub fn load_and_watch(&self) -> Result<(Config<T>, WatchHandle)> {
        let returned = self.load()?;

        let spawn_handle = self.spawn_watcher(returned.clone())?;
        Ok((returned, spawn_handle))
    }

    fn spawn_watcher(&self, config: Config<T>) -> Result<WatchHandle> {
        let files = self.files.clone();

        let mut stats = files
            .iter()
            .map(|f| f.mtime())
            .map_ok(Some)
            .collect::<Result<Vec<_>>>()?;

        let returned = WatchHandle::default();
        let dropped = returned.dropped.clone();
        std::thread::spawn(move || {
            let loader = Self {
                files,
                _p: Default::default(),
            };
            while !dropped.load(Ordering::Relaxed) {
                let new_stats = loader
                    .files
                    .iter()
                    .map(|f| f.mtime().ok())
                    .collect::<Vec<_>>();
                if &new_stats != &stats {
                    let _ = loader.load_value().and_then(|v| config.replace(v));
                }
                stats = new_stats;
                std::thread::sleep(WATCH_INTERVAL);
            }
        });
        Ok(returned)
    }
}

#[derive(Default)]
pub struct WatchHandle {
    dropped: Arc<AtomicBool>,
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Relaxed);
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

    fn mtime(&self) -> Result<SystemTime> {
        Ok(std::fs::metadata(self.path())?.modified()?)
    }
}
