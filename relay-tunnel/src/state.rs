use std::sync::Mutex;

use futures_util::Stream;
use relay_core::{Feed, TunnelMode};
use tokio::sync::Notify;

pub struct TunnelState {
    target: String,
    mode: TunnelMode,
    url: Mutex<String>,
    restart: Notify,
    feed: Feed<String>,
}

impl TunnelState {
    pub fn new(target: String, mode: TunnelMode) -> Self {
        Self { target, mode, url: Mutex::new(String::new()), restart: Notify::new(), feed: Feed::new(8) }
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn mode(&self) -> &TunnelMode {
        &self.mode
    }

    pub fn url(&self) -> String {
        self.url.lock().expect("tunnel url lock").clone()
    }

    pub fn set_url(&self, url: String) {
        *self.url.lock().expect("tunnel url lock") = url.clone();
        self.feed.push(url);
    }

    pub fn clear(&self) {
        self.url.lock().expect("tunnel url lock").clear();
        self.feed.push(String::new());
    }

    pub fn watch(&self) -> impl Stream<Item=String> + Send + 'static {
        self.feed.subscribe()
    }

    pub fn request_restart(&self) {
        self.restart.notify_one();
    }

    pub async fn wait_for_restart(&self) {
        self.restart.notified().await;
    }
}
