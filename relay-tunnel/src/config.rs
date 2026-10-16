use crate::TunnelMode;

#[derive(Clone)]
pub struct TunnelConfig {
    pub target: String,
    pub mode: TunnelMode,
}
