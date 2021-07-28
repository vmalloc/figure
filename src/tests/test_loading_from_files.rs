use super::utils::file_with;
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

    let value = cfg.get().value;
    assert_eq!(value, 2)
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

    let value = cfg.get().value;

    assert_eq!(value, 2)
}

//////////////////////////////////////////////////////////////////////////////////
