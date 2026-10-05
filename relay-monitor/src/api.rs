#[cfg(any(feature = "server", feature = "standalone"))]
use crate::Monitor;
use dioxus_fullstack::{JsonEncoding, ServerFnError, Streaming};

use crate::{Entry, Page};

#[haze::get("/api/monitor?page", monitor: Monitor)]
pub async fn watch(page: usize) -> Result<Streaming<Page<Entry>, JsonEncoding>, ServerFnError> {
    Ok(Streaming::new(monitor.pages(page)))
}

#[haze::server(monitor: Monitor)]
pub async fn clear_history() -> Result<(), ServerFnError> {
    monitor.clear().map_err(|error| ServerFnError::ServerError {
        message: error.message,
        code: 500,
        details: None,
    })
}
