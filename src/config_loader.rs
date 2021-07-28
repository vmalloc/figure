use crate::Config;
use anyhow::{Context, Result};
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
    spec: LoaderSpec,
    _p: PhantomData<T>,
}

#[derive(Clone, Default)]
struct LoaderSpec {
    files: Vec<FileSpec>,
    error_callbacks: Vec<Arc<dyn Fn(&anyhow::Error) + 'static + Send + Sync>>,
}

impl<T> ConfigLoader<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
{
    pub(crate) fn new(file: FileSpec) -> Self {
        let mut returned = Self {
            spec: Default::default(),
            _p: PhantomData::default(),
        };
        returned.spec.files.push(file);
        returned
    }

    pub fn on_watch_error(
        mut self,
        callback: impl Fn(&anyhow::Error) + Send + Sync + 'static,
    ) -> Self {
        self.spec.error_callbacks.push(Arc::new(callback));
        self
    }

    pub fn and_overlay_json(self, path: impl Into<PathBuf>) -> Self {
        self.and_overlay(FileSpec::Json(path.into()))
    }

    pub fn and_overlay_yaml(self, path: impl Into<PathBuf>) -> Self {
        self.and_overlay(FileSpec::Yaml(path.into()))
    }

    fn and_overlay(mut self, file: FileSpec) -> Self {
        self.spec.files.push(file);
        self
    }

    pub fn load(&self) -> Result<Config<T>> {
        let returned = Config::new_with(self.load_value()?)?;

        Ok(returned)
    }

    fn load_value(&self) -> Result<T> {
        let mut value = serde_json::Value::Null;

        for overlay in &self.spec.files {
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
        let spec = self.spec.clone();

        let mut stats = spec
            .files
            .iter()
            .map(|f| f.mtime())
            .collect::<Result<Vec<_>>>()?;

        let returned = WatchHandle::default();
        let dropped = returned.dropped.clone();
        std::thread::spawn(move || {
            let loader = Self {
                spec,
                _p: Default::default(),
            };
            while !dropped.load(Ordering::Relaxed) {
                if let Ok(Some(new_stats)) =
                    loader.reload_if_changed(&config, &stats).map_err(|e| {
                        log::error!("Error when watching for changes: {:?}", e);
                        loader.handle_error(e);
                    })
                {
                    stats = new_stats;
                }
                std::thread::sleep(WATCH_INTERVAL);
            }
        });
        Ok(returned)
    }

    fn reload_if_changed(
        &self,
        config: &Config<T>,
        stats: &[SystemTime],
    ) -> Result<Option<Vec<SystemTime>>> {
        let new_stats = dbg!(self
            .spec
            .files
            .iter()
            .map(|f| f.mtime())
            .collect::<Result<Vec<_>>>())?;
        if new_stats != stats {
            self.load_value()
                .context("Failed loading configuration from files")
                .and_then(|v| {
                    config
                        .replace(v)
                        .context("Failed replacing inner configuration value")
                })?;
            Ok(Some(new_stats))
        } else {
            Ok(None)
        }
    }

    fn handle_error(&self, error: anyhow::Error) {
        for handler in &self.spec.error_callbacks {
            handler(&error)
        }
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
        std::fs::metadata(self.path())
            .with_context(|| format!("Cannot fetch metadata for {:?}", self.path()))?
            .modified()
            .with_context(|| format!("Cannot get mtime for {:?}", self.path()))
    }
}
