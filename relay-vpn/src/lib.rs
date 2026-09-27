mod api;
#[cfg(feature = "server")]
mod controller;
#[cfg(feature = "server")]
mod service;
#[cfg(feature = "server")]
mod state;

pub use api::{VpnInfo, add_vpn, remove_vpn, vpns};

#[cfg(feature = "server")]
pub use api::VpnApi;
#[cfg(feature = "server")]
pub use controller::VpnController;
#[cfg(feature = "server")]
pub use service::Vpns;
#[cfg(feature = "server")]
pub use state::VpnState;
