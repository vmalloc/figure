use crate::{layer::Layer, Config};
use anyhow::{Context, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Clone)]
pub struct ConfigLoader<T>
where
    T: Send + Sync + 'static,
{
    spec: LoaderSpec<T>,
}

struct LoaderSpec<T> {
    layers: Vec<Layer>,
    factory: Option<Arc<dyn Fn() -> T + 'static + Send + Sync>>,
    error_callbacks: Vec<Arc<dyn Fn(&anyhow::Error) + 'static + Send + Sync>>,
}

// we have to implement this ourselves without derive, because deriving adds constraint `where T: Clone`
impl<T> Clone for LoaderSpec<T> {
    fn clone(&self) -> Self {
        Self {
            layers: self.layers.clone(),
            factory: self.factory.clone(),
            error_callbacks: self.error_callbacks.clone(),
        }
    }
}

impl<T> Default for LoaderSpec<T> {
    fn default() -> Self {
        Self {
            layers: Default::default(),
            factory: None,
            error_callbacks: Default::default(),
        }
    }
}

impl<T> ConfigLoader<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
{
    // we don't want loaders to be default-constructed elsewhere
    pub(crate) fn new() -> Self {
        Self {
            spec: Default::default(),
        }
    }

    pub(crate) fn with_factory(mut self, f: impl Fn() -> T + 'static + Send + Sync) -> Self {
        self.spec.factory.replace(Arc::new(f));
        self
    }

    pub fn on_watch_error(
        mut self,
        callback: impl Fn(&anyhow::Error) + Send + Sync + 'static,
    ) -> Self {
        self.spec.error_callbacks.push(Arc::new(callback));
        self
    }

    pub fn and_overlay_json(self, path: impl Into<PathBuf>) -> Self {
        self.and_overlay(Layer::json_file(path))
    }

    pub fn and_overlay_yaml(self, path: impl Into<PathBuf>) -> Self {
        self.and_overlay(Layer::yaml_file(path))
    }

    pub fn and_json_url(self, url: Url) -> Self {
        self.and_overlay(Layer::JsonUrl(url))
    }

    fn and_overlay(mut self, layer: Layer) -> Self {
        self.spec.layers.push(layer);
        self
    }

    pub fn load(&self) -> Result<Config<T>> {
        let returned = Config::new_with_raw(self.load_value()?)?;

        Ok(returned)
    }

    fn load_value(&self) -> Result<Value> {
        let mut value = match &self.spec.factory {
            Some(factory) => serde_json::to_value(&factory())?,
            None => serde_json::Value::Null,
        };

        for overlay_layer in &self.spec.layers {
            let overlay_value = overlay_layer.load()?;

            json_patch::merge(&mut value, &overlay_value);
        }

        Ok(value)
    }

    pub fn load_and_watch(&self, poll_interval: Duration) -> Result<(Config<T>, WatchHandle)> {
        let returned = self.load()?;

        let spawn_handle = self.spawn_watcher(returned.clone(), poll_interval)?;
        Ok((returned, spawn_handle))
    }

    fn spawn_watcher(&self, config: Config<T>, poll_interval: Duration) -> Result<WatchHandle>
    where
        Config<T>: Clone,
    {
        let spec = self.spec.clone();

        let mut prev_value = None;

        let returned = WatchHandle::default();
        let dropped = returned.dropped.clone();
        std::thread::spawn(move || {
            let loader = Self { spec };

            while !dropped.load(Ordering::Relaxed) {
                let _ = loader
                    .load_value()
                    .context("Failed loading configuration value")
                    .and_then(|value| {
                        if Some(&value) != prev_value.as_ref() {
                            config
                                .replace_raw(value.clone())
                                .context("Failed replacing value")?;
                            prev_value.replace(value);
                        }
                        Ok(())
                    })
                    .map_err(|e| loader.handle_error(e));

                std::thread::sleep(poll_interval);
            }
        });
        Ok(returned)
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
