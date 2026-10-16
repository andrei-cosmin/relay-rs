mod api;
#[cfg(feature = "backend")]
mod rules;
#[cfg(feature = "backend")]
mod store;
mod target_list;

pub use api::{save_targets, targets};
#[cfg(feature = "backend")]
pub use rules::Rules;
#[cfg(feature = "backend")]
pub use store::TargetStore;
pub use target_list::TargetList;
