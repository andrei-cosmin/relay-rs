use std::path::PathBuf;

use relay_core::DataDir;

#[derive(Clone)]
pub struct VpnConfig {
    pub dir: PathBuf,
}

#[haze::resource]
fn vpn_config(dir: DataDir) -> VpnConfig {
    VpnConfig {
        dir: dir.0.join("vpn"),
    }
}
