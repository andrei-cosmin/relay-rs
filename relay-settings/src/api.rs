use dioxus_fullstack::{ServerFnError, server};
#[cfg(feature = "server")]
use dioxus_server::axum::Extension;
#[cfg(feature = "server")]
use std::sync::Arc;

use crate::Settings;

#[cfg(feature = "server")]
pub trait SettingsApi: Send + Sync {
    fn settings(&self) -> Settings;
    fn save(&self, settings: Settings) -> Result<(), String>;
    fn restart(&self);
}

#[server(api: Extension<Arc<dyn SettingsApi>>)]
pub async fn settings() -> Result<Settings, ServerFnError> {
    Ok(api.settings())
}

#[server(api: Extension<Arc<dyn SettingsApi>>)]
pub async fn save_settings(settings: Settings) -> Result<(), ServerFnError> {
    api.save(settings).map_err(|message| ServerFnError::ServerError { message, code: 400, details: None })
}

#[server(api: Extension<Arc<dyn SettingsApi>>)]
pub async fn restart() -> Result<(), ServerFnError> {
    api.restart();
    Ok(())
}
