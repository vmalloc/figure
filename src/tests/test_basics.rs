use crate::Config;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[test]
fn test_construct_empty() {
    let _cfg = Config::empty();
}

#[test]
fn test_get_set() {
    let cfg = Config::<Value>::new_with(json!(
        {
            "x": {
                "y": 3
            }
        }
    ))
    .unwrap();

    assert_eq!(cfg.get_raw::<u32>("x.y").unwrap(), 3);
    cfg.set_raw("x.y", 2).unwrap();
    assert_eq!(cfg.get_raw::<u32>("x.y").unwrap(), 2);
}

#[test]
fn test_object_config() {
    #[derive(Deserialize, Serialize)]
    struct SampleConfig {
        value: u32,
    }

    let cfg = Config::<SampleConfig>::new_with(SampleConfig { value: 2 }).unwrap();
    assert_eq!(cfg.get_raw::<u32>("value").unwrap(), 2);
}

#[test]
fn test_clone() {
    let cfg = Config::empty();
    let inner = cfg.get().clone();
    assert_eq!(inner, json!({}))
}
