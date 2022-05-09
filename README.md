**Note**: figure is still in very early stages of development. Use at your own risk

# Overview

`figure` is a crate for managing configuration for Rust applications. It puts emphasis on ergonomics in realistic use cases:

1. Configuration is `Clone`, and centrally controlled - this allows you to pass it around your app to whichever components need it without worrying too much
2. Live-reloading is supported out-of-the-box, which is useful in cases like mapping a Kubernetes ConfigMap as your runtime configuration
3. Configurations work in "overlays", meaning there's a separation between the default values and the updates you apply on top of them. This allows you to reset values, or preserve your updates when the underlying configuration is reloaded

# Getting Started

`figure` Configuration objects are wrappers around an inner configuration value. By default this value is a `serde_json::Value`, but this can be customized to fit any type that is `Serialize` and `Deserialize`. 

Configuration objects maintain the "raw" state of the configuration they hold - i.e. the serde_json representation of the value before building it. Manipulating this data is done by `set_raw` and `get_raw`:

```rust
use figure::Config;

let cfg = Config::empty();
cfg.set_raw("x", 2).unwrap();
let value: u32 = cfg.get_raw("x").unwrap();
assert_eq!(value, 2);
```

More useful cases involve creating your own types for holding the "built" configuration values. In this case it makes more sense to get the "built" configuration value via `get()`, and modify it via `modify()`:

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
cfg.modify(|inner| inner.value=2);
```