use dioxus_fullstack::ServerFnError;

use crate::VpnInfo;
#[cfg(any(feature = "server", feature = "standalone"))]
use crate::Vpns;

#[haze::server(handle: Vpns)]
pub async fn vpns() -> Result<Vec<VpnInfo>, ServerFnError> {
    Ok(handle.list())
}

#[haze::server(handle: Vpns)]
pub async fn add_vpn(name: String, config: String) -> Result<(), ServerFnError> {
    handle
        .add(name, config)
        .map_err(|message| ServerFnError::ServerError {
            message,
            code: 400,
            details: None,
        })
}

#[haze::server(handle: Vpns)]
pub async fn remove_vpn(name: String) -> Result<(), ServerFnError> {
    handle
        .remove(name)
        .map_err(|message| ServerFnError::ServerError {
            message,
            code: 400,
            details: None,
        })
}
