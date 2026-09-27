mod api;
#[cfg(feature = "server")]
mod controller;
#[cfg(feature = "server")]
mod rules;

pub use api::{save_targets, targets};

#[cfg(feature = "server")]
pub use api::TargetsApi;
#[cfg(feature = "server")]
pub use controller::TargetsController;
#[cfg(feature = "server")]
pub use rules::Rules;
