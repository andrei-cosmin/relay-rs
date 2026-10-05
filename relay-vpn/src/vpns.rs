use std::{net::SocketAddr, path::Path, sync::Arc};

use haze::Pack;

use crate::{VpnConfig, VpnInfo, supervisor::Supervisor};

#[derive(Clone, Pack)]
pub struct Vpns {
    config: VpnConfig,
    #[pack(func = Supervisor::spawn(&config))]
    supervisor: Arc<Supervisor>,
}

impl Vpns {
    pub fn dir(&self) -> &Path {
        &self.config.dir
    }

    pub fn list(&self) -> Vec<VpnInfo> {
        self.supervisor.list()
    }

    pub fn add(&self, name: String, config: String) -> Result<(), String> {
        self.supervisor.add(name, config)
    }

    pub fn remove(&self, name: String) -> Result<(), String> {
        self.supervisor.remove(name)
    }

    pub fn socks_address(&self, name: &str) -> Option<SocketAddr> {
        self.supervisor.socks_address(name)
    }
}

#[cfg(test)]
mod tests {
    use haze::Resources;

    use super::*;

    #[tokio::test]
    async fn an_empty_directory_lists_no_vpns() {
        let dir = std::env::temp_dir().join(format!("relay-vpns-{}", std::process::id()));
        let mut resources = Resources::new();
        resources.insert(VpnConfig { dir: dir.clone() });
        let vpns = Vpns::build(&resources).unwrap();
        assert_eq!(vpns.dir(), dir);
        assert!(vpns.list().is_empty());
        assert_eq!(vpns.socks_address("Warp"), None);
    }
}
