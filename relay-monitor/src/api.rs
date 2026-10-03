#[cfg(feature = "server")]
use crate::Monitor;
use dioxus_fullstack::{ServerEvents, ServerFnError};

use crate::{Entry, Page};

#[haze::get("/api/monitor?page", monitor: Monitor)]
pub async fn watch(page: usize) -> Result<ServerEvents<Page<Entry>>, ServerFnError> {
    Ok(ServerEvents::new(move |sender| async move {
        monitor.follow(page, sender).await
    }))
}

#[haze::server(monitor: Monitor)]
pub async fn clear_history() -> Result<(), ServerFnError> {
    monitor.clear().map_err(|error| ServerFnError::ServerError {
        message: error.message,
        code: 500,
        details: None,
    })
}
