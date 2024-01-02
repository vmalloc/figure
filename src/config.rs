use anyhow::{format_err, Result};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

use crate::config_loader::ConfigLoader;

struct ConfigInner<T: Send + Sync> {
    built: T,
    raw: Value,
    overlay: Value,
}

impl<T> ConfigInner<T>
where
    T: Send + Sync + for<'de> Deserialize<'de>,
{
    fn rebuild(&mut self) -> Result<()> {
        let mut to_build = self.raw.clone();
        json_patch::merge(&mut to_build, &self.overlay);

        self.built = serde_json::from_value(to_build)?;
        Ok(())
    }
}

pub struct Config<T: Send + Sync> {
    inner: Arc<RwLock<ConfigInner<T>>>,
}

impl<T: Send + Sync> Clone for Config<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Config<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
{
    pub fn load_json_file(path: impl Into<PathBuf>) -> ConfigLoader<T> {
        ConfigLoader::new().and_overlay_json(path)
    }

    pub fn load_yaml_file(path: impl Into<PathBuf>) -> ConfigLoader<T> {
        ConfigLoader::new().and_overlay_yaml(path)
    }

    // Gets a raw value by its path
    pub fn get_raw<V>(&self, path: &str) -> Result<V>
    where
        V: for<'de> Deserialize<'de>,
    {
        let locked = self.read_inner();
        let mut returned_raw = &locked.raw;
        let mut returned_overlay = &locked.overlay;
        for part in path.split('.') {
            returned_raw = &returned_raw[part];
            returned_overlay = &returned_overlay[part];
        }

        let value = if returned_overlay.is_null() {
            returned_raw
        } else {
            returned_overlay
        };

        let returned = serde_json::from_value(value.clone())?;
        Ok(returned)
    }

    pub fn set_raw<V>(&self, path: &str, value: V) -> Result<()>
    where
        V: Serialize,
    {
        let mut parts = path.split('.').rev();
        let attr = parts
            .next()
            .ok_or_else(|| format_err!("Invalid attribute string"))?;
        let mut patch = json!({attr: serde_json::to_value(&value)?});

        for part in parts {
            patch = json!({ part: patch });
        }
        self.merge(patch)
    }

    pub fn get(&self) -> MappedRwLockReadGuard<T> {
        RwLockReadGuard::map(self.inner.read(), |inner| &inner.built)
    }

    fn merge(&self, patch: Value) -> Result<()> {
        self.merge_and_keep_locked(patch).map(drop)
    }

    fn merge_and_keep_locked(&self, patch: Value) -> Result<RwLockWriteGuard<ConfigInner<T>>> {
        let mut locked = self.write_inner();
        let mut new_overlay = locked.overlay.clone();
        json_patch::merge(&mut new_overlay, &patch);
        locked.overlay = new_overlay;
        locked.rebuild()?;
        Ok(locked)
    }

    fn read_inner(&self) -> RwLockReadGuard<ConfigInner<T>> {
        self.inner.read()
    }

    fn write_inner(&self) -> RwLockWriteGuard<ConfigInner<T>> {
        self.inner.write()
    }

    pub fn replace_raw(&self, value: Value) -> Result<()> {
        let raw = serde_json::to_value(value)?;
        let mut locked = self.write_inner();
        locked.raw = raw;
        locked.rebuild()?;

        Ok(())
    }
}

impl Config<Value> {
    pub fn empty() -> Self {
        Self::new_with(json!({})).unwrap()
    }
}

impl<T> Config<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
{
    pub fn new_with(built: T) -> Result<Self> {
        let raw = serde_json::to_value(&built)?;
        Ok(Self::new_with_raw_and_built(raw, built))
    }

    pub(crate) fn new_with_raw(raw: Value) -> Result<Self> {
        let built = serde_json::from_value(raw.clone())?;
        Ok(Self::new_with_raw_and_built(raw, built))
    }

    fn new_with_raw_and_built(raw: Value, built: T) -> Self {
        let overlay = json!({});
        Self {
            inner: Arc::new(RwLock::new(ConfigInner {
                built,
                raw,
                overlay,
            })),
        }
    }
}

impl<T> Config<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Default + Send + Sync,
{
    pub fn new_with_default() -> Result<Self> {
        Self::new_with(Default::default())
    }

    pub fn load_default() -> ConfigLoader<T> {
        ConfigLoader::new().with_factory(Default::default)
    }
}
