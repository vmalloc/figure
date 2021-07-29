use super::utils::file_with;
use crate::Config;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ExampleConfig {
    name: String,
    id: u8,
}

#[test]
fn test_overlays() {
    let (_, root_file) = file_with("name: Test");

    let (_, overlay_file1) = file_with("id: 3");

    let (_, overlay_file2) = file_with("name: Another Test");

    let cfg = Config::<ExampleConfig>::load_yaml_file(&root_file)
        .and_overlay_yaml(&overlay_file1)
        .and_overlay_yaml(&overlay_file2)
        .load()
        .unwrap();
    let inner = cfg.get();
    assert_eq!(inner.name, "Another Test");
    assert_eq!(inner.id, 3);
}

#[test]
fn test_overlay_with_default() {
    #[derive(Default, Deserialize, Serialize)]

    pub struct Example {
        name: String,
        id: u32,
    }

    let (_, overlay_path) = file_with("id: 666");

    let cfg = Config::<Example>::load_default()
        .and_overlay_yaml(&overlay_path)
        .load()
        .unwrap();
    assert_eq!(cfg.get().name, "");
    assert_eq!(cfg.get().id, 666);
}
