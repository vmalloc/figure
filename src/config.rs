use anyhow::{format_err, Result};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

use crate::config_loader::ConfigLoader;

struct ConfigInner<T: Send + Sync> {
    value: T,
    raw: Value,
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
        let mut returned = &locked.raw;
        for part in path.split('.') {
            returned = &returned[part];
        }
        let returned = serde_json::from_value(returned.clone())?;
        Ok(returned)
    }

    // Sets a raw value by its path
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
        RwLockReadGuard::map(self.inner.read(), |inner| &inner.value)
    }

    fn merge(&self, patch: Value) -> Result<()> {
        self.merge_and_keep_locked(patch).map(drop)
    }

    fn merge_and_keep_locked(&self, patch: Value) -> Result<RwLockWriteGuard<ConfigInner<T>>> {
        let mut locked = self.write_inner();
        let mut new_raw = locked.raw.clone();
        json_patch::merge(&mut new_raw, &patch);
        let new_value = serde_json::from_value(new_raw.clone())?;

        locked.raw = new_raw;
        locked.value = new_value;
        Ok(locked)
    }

    fn read_inner(&self) -> RwLockReadGuard<ConfigInner<T>> {
        self.inner.read()
    }

    fn write_inner(&self) -> RwLockWriteGuard<ConfigInner<T>> {
        self.inner.write()
    }

    pub fn replace(&self, value: T) -> Result<()> {
        let raw = serde_json::to_value(&value)?;
        let mut locked = self.write_inner();
        locked.value = value;
        locked.raw = raw;

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
    pub fn new_with(value: T) -> Result<Self> {
        let raw = serde_json::to_value(&value)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(ConfigInner { value, raw })),
        })
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
