use super::utils::{file_with, short_sleep};
use crate::{config_loader::ConfigLoader, Config};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::{Seek, SeekFrom, Write},
    sync::Arc,
    time::Duration,
};

#[derive(Deserialize, Serialize)]
struct ExampleConfig {
    name: String,
    id: u8,
}

#[test]
fn test_watching_changes_simple_files() {
    let (mut root_file, root_path) = file_with("name: name");

    let (mut overlay_file, overlay_path) = file_with("id: 3");

    let (cfg, _watcher) = Config::<ExampleConfig>::load_yaml_file(&root_path)
        .and_overlay_yaml(&overlay_path)
        .load_and_watch(Duration::from_millis(1))
        .unwrap();
    {
        let inner = cfg.get();
        assert_eq!(inner.name, "name");
        assert_eq!(inner.id, 3);
    }

    root_file.seek(SeekFrom::Start(0)).unwrap();
    root_file.write_all("name: new_name".as_bytes()).unwrap();
    root_file.flush().unwrap();

    short_sleep();

    assert_eq!(cfg.get().name, "new_name");
    assert_eq!(cfg.get().id, 3);

    overlay_file.seek(SeekFrom::Start(0)).unwrap();
    overlay_file.write_all("id: 6".as_bytes()).unwrap();
    overlay_file.flush().unwrap();

    short_sleep();

    assert_eq!(cfg.get().name, "new_name");
    assert_eq!(cfg.get().id, 6);
}

#[test]
fn test_watching_changes_files_and_url() {
    let server = super::utils::http_server_with(r#"{}"#).unwrap();
    let (mut overlay_file, overlay_path) = file_with(r#"{}"#);

    let (cfg, _watcher) = ConfigLoader::<Value>::new()
        .and_overlay_json(&overlay_path)
        .and_json_url(server.url().clone())
        .load_and_watch(Duration::from_millis(1))
        .unwrap();

    assert_eq!(*cfg.get(), json!({}));

    server.set_contents(r#"{"a": 1}"#);

    short_sleep();

    assert_eq!(*cfg.get(), json!({"a": 1}));

    overlay_file.seek(SeekFrom::Start(0)).unwrap();
    overlay_file.write_all(r#"{"b": 2}"#.as_bytes()).unwrap();
    overlay_file.flush().unwrap();

    short_sleep();

    assert_eq!(*cfg.get(), json!({"a": 1, "b": 2}));
}

#[test]
fn test_watching_changes_with_set_overrides() {
    let (mut root_file, root_path) = file_with("name: name");
    let (mut overlay_file, overlay_path) = file_with("id: 3");

    let (cfg, _watcher) = Config::<ExampleConfig>::load_yaml_file(&root_path)
        .and_overlay_yaml(&overlay_path)
        .load_and_watch(Duration::from_millis(1))
        .unwrap();
    {
        let inner = cfg.get();
        assert_eq!(inner.name, "name");
        assert_eq!(inner.id, 3);
    }

    cfg.set_raw("name", "other name").unwrap();

    root_file.seek(SeekFrom::Start(0)).unwrap();
    root_file.write_all("name: new_name".as_bytes()).unwrap();
    root_file.flush().unwrap();

    short_sleep();

    assert_eq!(cfg.get().name, "other name");
    assert_eq!(cfg.get().id, 3);

    overlay_file.seek(SeekFrom::Start(0)).unwrap();
    overlay_file.write_all("id: 6".as_bytes()).unwrap();
    overlay_file.flush().unwrap();

    short_sleep();

    assert_eq!(cfg.get().name, "other name");
    assert_eq!(cfg.get().id, 6);
}

#[cfg(any(target_os = "unix", target_os = "macos"))]
#[test]
fn test_watching_through_symlinks() {
    // In k8s (for example) side-loaded configuration is implemented as symlinks to an underlying mount.
    // When the configuration changes,
    // the symlink changes, and not the file itself

    let temp_directory = tempfile::tempdir().unwrap();

    let dir_1 = temp_directory.path().join("dir1");
    let dir_2 = temp_directory.path().join("dir2");
    std::fs::create_dir_all(&dir_1).unwrap();
    std::fs::create_dir_all(&dir_2).unwrap();
    let symlink_path = temp_directory.path().join("symlink");
    std::os::unix::fs::symlink(&dir_1, &symlink_path).unwrap();

    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(dir_1.join("file"))
        .unwrap()
        .write_all("id: 1".as_bytes())
        .unwrap();
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(dir_2.join("file"))
        .unwrap()
        .write_all("id: 2".as_bytes())
        .unwrap();

    let (_overlay, path) = file_with("name: Test");
    let (cfg, _watcher) = Config::<ExampleConfig>::load_yaml_file(symlink_path.join("file"))
        .and_overlay_yaml(&path)
        .load_and_watch(Duration::from_millis(1))
        .unwrap();

    assert_eq!(cfg.get().name, "Test");
    assert_eq!(cfg.get().id, 1);

    std::fs::remove_file(&symlink_path).unwrap();
    std::os::unix::fs::symlink(&dir_2, &symlink_path).unwrap();
    short_sleep();

    assert_eq!(cfg.get().name, "Test");
    assert_eq!(cfg.get().id, 2);
}

#[test]
fn test_watching_changes_errors() {
    let errors = Arc::new(Mutex::new(Vec::new()));
    let errors_clone = errors.clone();
    let (file, path) = file_with("name: name\nid: 1");
    let (_cfg, _watcher) = Config::<ExampleConfig>::load_yaml_file(&path)
        .on_watch_error(move |e| errors_clone.lock().push(format!("{:?}", e)))
        .load_and_watch(Duration::from_millis(1))
        .unwrap();

    drop(file);
    path.close().unwrap();
    short_sleep();

    let locked_errors = errors.lock();
    assert!(locked_errors.len() > 0, "Errors not detected as expected!");
    for error in &*locked_errors {
        assert!(
            format!("{:?}", error)
                .to_ascii_lowercase()
                .contains("no such file"),
            "Unexpected error: {:?}",
            error
        );
    }
}
