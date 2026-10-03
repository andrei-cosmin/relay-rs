use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
pub struct Vpn {
    pub port: u16,
    pub up: bool,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct VpnInfo {
    pub name: String,
    pub up: bool,
}
