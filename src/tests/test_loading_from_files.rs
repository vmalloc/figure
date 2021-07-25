use std::{fs::File, io::Write};
use tempfile::{NamedTempFile, TempPath};

use crate::Config;

#[derive(serde::Deserialize, serde::Serialize)]
struct ExampleConfig {
    value: u32,
}

#[test]
fn test_loading_json_file() {
    let (_, path) = file_with(
        r#"
    {
        "value": 2
    }
    
    "#,
    );
    let cfg = Config::<ExampleConfig>::from_json_file(&path)
        .load()
        .unwrap();

    let inner = cfg.get();

    assert_eq!(inner.value, 2)
}

#[test]
fn test_loading_yaml_file() {
    let (_, path) = file_with(
        r#"
value: 2
"#,
    );
    let cfg = Config::<ExampleConfig>::from_yaml_file(&path)
        .load()
        .unwrap();

    let inner = cfg.get();

    assert_eq!(inner.value, 2)
}

//////////////////////////////////////////////////////////////////////////////////

fn file_with(contents: &str) -> (File, TempPath) {
    let mut returned = NamedTempFile::new().unwrap();

    returned
        .write_all(contents.as_bytes())
        .expect("Writing to file failed");
    returned.into_parts()
}
