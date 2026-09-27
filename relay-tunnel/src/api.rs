use dioxus_fullstack::{JsonEncoding, ServerFnError, Streaming, server};
#[cfg(feature = "server")]
use dioxus_server::axum::Extension;
#[cfg(feature = "server")]
use std::sync::Arc;

#[cfg(feature = "server")]
pub trait TunnelApi: Send + Sync {
    fn url(&self) -> String;
    fn enabled(&self) -> bool;
    fn restart(&self);
    fn watch(&self) -> Streaming<String, JsonEncoding>;
}

#[server(api: Extension<Arc<dyn TunnelApi>>)]
pub async fn tunnel() -> Result<String, ServerFnError> {
    Ok(api.url())
}

#[server(api: Extension<Arc<dyn TunnelApi>>)]
pub async fn tunnel_enabled() -> Result<bool, ServerFnError> {
    Ok(api.enabled())
}

#[server(api: Extension<Arc<dyn TunnelApi>>)]
pub async fn restart_tunnel() -> Result<(), ServerFnError> {
    api.restart();
    Ok(())
}

#[server(api: Extension<Arc<dyn TunnelApi>>)]
pub async fn watch_tunnel() -> Result<Streaming<String, JsonEncoding>, ServerFnError> {
    Ok(api.watch())
}
