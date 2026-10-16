#[cfg(any(feature = "server", feature = "standalone"))]
use crate::Tunnel;
use dioxus_fullstack::{JsonEncoding, ServerFnError, Streaming};

#[haze::server(handle: Tunnel)]
pub async fn tunnel() -> Result<String, ServerFnError> {
    Ok(handle.current_url())
}

#[haze::server(handle: Tunnel)]
pub async fn tunnel_enabled() -> Result<bool, ServerFnError> {
    Ok(handle.is_enabled())
}

#[haze::server(handle: Tunnel)]
pub async fn restart_tunnel() -> Result<(), ServerFnError> {
    handle.request_restart();
    Ok(())
}

#[haze::server(handle: Tunnel)]
pub async fn watch_tunnel() -> Result<Streaming<String, JsonEncoding>, ServerFnError> {
    Ok(Streaming::new(handle.url_updates()))
}
