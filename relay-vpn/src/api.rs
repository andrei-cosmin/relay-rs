use dioxus_fullstack::{ServerFnError, server};
#[cfg(feature = "server")]
use dioxus_server::axum::Extension;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use std::sync::Arc;

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct VpnInfo {
    pub name: String,
    pub up: bool,
}

#[cfg(feature = "server")]
pub trait VpnApi: Send + Sync {
    fn list(&self) -> Vec<VpnInfo>;
    fn add(&self, name: String, config: String) -> Result<(), String>;
    fn remove(&self, name: String) -> Result<(), String>;
}

#[server(api: Extension<Arc<dyn VpnApi>>)]
pub async fn vpns() -> Result<Vec<VpnInfo>, ServerFnError> {
    Ok(api.list())
}

#[server(api: Extension<Arc<dyn VpnApi>>)]
pub async fn add_vpn(name: String, config: String) -> Result<(), ServerFnError> {
    api.add(name, config).map_err(|message| ServerFnError::ServerError { message, code: 400, details: None })
}

#[server(api: Extension<Arc<dyn VpnApi>>)]
pub async fn remove_vpn(name: String) -> Result<(), ServerFnError> {
    api.remove(name).map_err(|message| ServerFnError::ServerError { message, code: 400, details: None })
}
