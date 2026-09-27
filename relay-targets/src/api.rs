use dioxus_fullstack::{ServerFnError, server};
#[cfg(feature = "server")]
use dioxus_server::axum::Extension;
use relay_core::TargetList;
#[cfg(feature = "server")]
use std::sync::Arc;

#[cfg(feature = "server")]
pub trait TargetsApi: Send + Sync {
    fn targets(&self) -> TargetList;
    fn save(&self, targets: TargetList) -> Result<(), String>;
}

#[server(api: Extension<Arc<dyn TargetsApi>>)]
pub async fn targets() -> Result<TargetList, ServerFnError> {
    Ok(api.targets())
}

#[server(api: Extension<Arc<dyn TargetsApi>>)]
pub async fn save_targets(targets: TargetList) -> Result<(), ServerFnError> {
    api.save(targets).map_err(|message| ServerFnError::ServerError { message, code: 400, details: None })
}
