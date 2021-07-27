use std::{
    io::{Seek, SeekFrom, Write},
    time::Duration,
};

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

    let cfg = Config::<ExampleConfig>::from_yaml_file(&root_file)
        .and_overlay_yaml(&overlay_file1)
        .and_overlay_yaml(&overlay_file2)
        .load()
        .unwrap();
    let inner = cfg.get();
    assert_eq!(inner.name, "Another Test");
    assert_eq!(inner.id, 3);
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

    std::thread::sleep(Duration::from_millis(10));

    {
        let inner = cfg.get();
        assert_eq!(inner.name, "new_name");
        assert_eq!(inner.id, 3);
    }

    overlay_file.seek(SeekFrom::Start(0)).unwrap();
    overlay_file.write_all("id: 6".as_bytes()).unwrap();
    overlay_file.flush().unwrap();

    std::thread::sleep(Duration::from_millis(10));

    {
        let inner = cfg.get();
        assert_eq!(inner.name, "new_name");
        assert_eq!(inner.id, 6);
    }
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

    {
        let inner = cfg.get();
        assert_eq!(inner.name, "Test");
        assert_eq!(inner.id, 1);
    }

    std::fs::remove_file(&symlink_path).unwrap();
    std::os::unix::fs::symlink(&dir_2, &symlink_path).unwrap();
    std::thread::sleep(Duration::from_millis(10));

    {
        let inner = cfg.get();
        assert_eq!(inner.name, "Test");
        assert_eq!(inner.id, 2);
    }
}
