mod api;
#[cfg(feature = "backend")]
mod config;
#[cfg(feature = "backend")]
mod supervisor;
mod vpn;
#[cfg(feature = "backend")]
mod vpns;

pub use api::{add_vpn, remove_vpn, vpns};
#[cfg(feature = "backend")]
pub use config::VpnConfig;
#[cfg(feature = "backend")]
use vpn::Vpn;
pub use vpn::VpnInfo;
#[cfg(feature = "backend")]
pub use vpns::Vpns;
