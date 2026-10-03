mod api;
#[cfg(feature = "server")]
mod config;
#[cfg(feature = "server")]
mod supervisor;
mod vpn;
#[cfg(feature = "server")]
mod vpns;

pub use api::{add_vpn, remove_vpn, vpns};
#[cfg(feature = "server")]
pub use config::VpnConfig;
#[cfg(feature = "server")]
use vpn::Vpn;
pub use vpn::VpnInfo;
#[cfg(feature = "server")]
pub use vpns::Vpns;
