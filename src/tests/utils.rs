use std::{fs::File, io::Write, time::Duration};
use tempfile::{NamedTempFile, TempPath};

pub(super) fn file_with(contents: &str) -> (File, TempPath) {
    let mut returned = NamedTempFile::new().unwrap();

    returned
        .write_all(contents.as_bytes())
        .expect("Writing to file failed");
    returned.into_parts()
}

pub(super) fn short_sleep() {
    std::thread::sleep(Duration::from_millis(50))
}
