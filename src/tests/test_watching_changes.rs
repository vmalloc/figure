use super::utils::{file_with, short_sleep};
use crate::Config;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    io::{Seek, SeekFrom, Write},
    sync::Arc,
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

    let (cfg, _watcher) = Config::<ExampleConfig>::from_yaml_file(&root_path)
        .and_overlay_yaml(&overlay_path)
        .load_and_watch()
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
    let (cfg, _watcher) = Config::<ExampleConfig>::from_yaml_file(symlink_path.join("file"))
        .and_overlay_yaml(&path)
        .load_and_watch()
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
    let (_cfg, _watcher) = Config::<ExampleConfig>::from_yaml_file(&path)
        .on_watch_error(move |e| errors_clone.lock().push(format!("{:?}", e)))
        .load_and_watch()
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
