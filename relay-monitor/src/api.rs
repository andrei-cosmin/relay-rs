#[cfg(any(feature = "server", feature = "standalone"))]
use crate::Monitor;
use dioxus_fullstack::{JsonEncoding, ServerFnError, Streaming};

use crate::{Entry, Page};

#[haze::get("/api/monitor/changes", monitor: Monitor)]
pub async fn changes() -> Result<Streaming<u64, JsonEncoding>, ServerFnError> {
    Ok(Streaming::new(monitor.changes()))
}

#[haze::get("/api/monitor?page", monitor: Monitor)]
pub async fn entries(page: usize) -> Result<Page<Entry>, ServerFnError> {
    monitor
        .page(page)
        .map_err(|error| ServerFnError::ServerError {
            message: error.message,
            code: 500,
            details: None,
        })
}

#[haze::server(monitor: Monitor)]
pub async fn clear_history() -> Result<(), ServerFnError> {
    monitor.clear().map_err(|error| ServerFnError::ServerError {
        message: error.message,
        code: 500,
        details: None,
    })
}
