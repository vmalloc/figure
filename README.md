**Note**: figure is still in very early stages of development. Use at your own risk

# Overview

`figure` is a crate for managing configuration for Rust applications. It puts emphasis on ergonomics in realistic use cases:

1. Configuration is `Clone`, and centrally controlled - this allows you to pass it around your app to whichever components need it without worrying too much
2. Live-reloading is supported out-of-the-box, which is useful in cases like mapping a Kubernetes ConfigMap as your runtime configuration
3. Configurations work in "overlays", meaning there's a separation between the default values and the updates you apply on top of them. This allows you to reset values, or preserve your updates when the underlying configuration is reloaded

# Getting Started

The most basic use case is using and populating an empty configuration:

```rust
use figure::Config;

let cfg = Config::empty();
cfg.set_raw("x", 2).unwrap();
let value: u32 = cfg.get_raw("x").unwrap();
assert_eq!(value, 2);
```

The "raw" in the above example means that the updates happen on the raw structure of the configuration, which is actually based on `serde_json::Value`. In the default case this is has little meaning, but one you use `figure` to hold your own custom data types this makes more sense:

```rust
use figure::Config;

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct MyConfig {
    value: u32,
}


let cfg = Config::<MyConfig>::new_with_default().unwrap();
let value = cfg.get().value;
assert_eq!(value, 0);
cfg.set_raw("value", 2);
let value = cfg.get().value;
assert_eq!(value, 2);
```