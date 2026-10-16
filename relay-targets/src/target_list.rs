use relay_core::Target;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct TargetList {
    #[serde(default)]
    pub targets: Vec<Target>,
}
