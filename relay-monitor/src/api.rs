use dioxus_fullstack::{JsonEncoding, ServerFnError, Streaming, server};

use crate::{Entry, History};
#[cfg(feature = "server")]
use dioxus_server::axum::Extension;
#[cfg(feature = "server")]
use std::sync::Arc;

#[cfg(feature = "server")]
pub trait MonitorApi: Send + Sync {
    fn history(&self, page: usize) -> Result<History, String>;
    fn clear(&self) -> Result<(), String>;
    fn watch(&self) -> Streaming<Entry, JsonEncoding>;
}

#[server(api: Extension<Arc<dyn MonitorApi>>)]
pub async fn history(page: usize) -> Result<History, ServerFnError> {
    api.history(page).map_err(|message| ServerFnError::ServerError { message, code: 500, details: None })
}

#[server(api: Extension<Arc<dyn MonitorApi>>)]
pub async fn clear_history() -> Result<(), ServerFnError> {
    api.clear().map_err(|message| ServerFnError::ServerError { message, code: 500, details: None })
}

#[server(api: Extension<Arc<dyn MonitorApi>>)]
pub async fn watch() -> Result<Streaming<Entry, JsonEncoding>, ServerFnError> {
    Ok(api.watch())
}
