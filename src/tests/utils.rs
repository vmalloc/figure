use parking_lot::Mutex;
use reqwest::Url;
use std::{
    fs::File,
    io::Write,
    sync::{mpsc::channel, Arc},
    time::Duration,
};
use tempfile::{NamedTempFile, TempPath};

pub(super) fn file_with(contents: &str) -> (File, TempPath) {
    let mut returned = NamedTempFile::new().unwrap();

    returned
        .write_all(contents.as_bytes())
        .expect("Writing to file failed");
    returned.into_parts()
}

pub(super) fn short_sleep() {
    std::thread::sleep(short_duration())
}

fn short_duration() -> Duration {
    Duration::from_millis(50)
}

pub(super) fn http_server_with(contents: impl Into<String>) -> anyhow::Result<MockServer> {
    let (tx, rx) = channel();
    let contents = Arc::new(Mutex::new(contents.into()));

    let contents_clone = contents.clone();

    std::thread::spawn(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let app = axum::Router::new().route(
                    "/contents",
                    axum::routing::get(move || async move { contents_clone.lock().clone() }),
                );
                let server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
                    .serve(app.into_make_service());
                tx.send(server.local_addr()).unwrap();
                server.await.unwrap()
            })
    });

    let addr = rx.recv().unwrap();

    let url = format!("http://{addr}/contents").parse()?;

    Ok(MockServer { contents, url })
}

pub(super) struct MockServer {
    contents: Arc<Mutex<String>>,
    url: Url,
}

impl MockServer {
    pub(super) fn set_contents(&self, new_contents: impl Into<String>) {
        *self.contents.lock() = new_contents.into();
    }

    /// Get a reference to the mock server's url.
    #[must_use]
    pub(super) fn url(&self) -> &Url {
        &self.url
    }
}
