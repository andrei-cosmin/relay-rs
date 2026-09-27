use std::net::{Ipv4Addr, SocketAddr};

use relay_core::TunnelMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub listen: SocketAddr,
    pub admin: SocketAddr,
    pub tunnel: TunnelMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 8585)),
            admin: SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080)),
            tunnel: TunnelMode::default(),
        }
    }
}
