use serde::{Deserialize, Serialize};

#[derive(Clone, Default, PartialEq, Deserialize, Serialize)]
pub enum TunnelMode {
    Off,
    #[default]
    Quick,
    Domain {
        token: String,
    },
}

impl TunnelMode {
    pub fn enabled(&self) -> bool {
        !matches!(self, Self::Off)
    }
}
