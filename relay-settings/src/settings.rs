use std::net::{Ipv4Addr, SocketAddr};

use relay_tunnel::TunnelMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub listen: SocketAddr,
    pub tunnel: TunnelMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 8585)),
            tunnel: TunnelMode::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use relay_tunnel::TunnelMode;

    use super::*;

    #[test]
    fn defaults_are_a_local_proxy_and_a_quick_tunnel() {
        let settings = Settings::default();
        assert_eq!(settings.listen.to_string(), "127.0.0.1:8585");
        assert!(settings.tunnel == TunnelMode::Quick);
    }

    #[test]
    fn an_old_file_with_an_admin_address_and_no_tunnel_still_loads() {
        let settings: Settings =
            ron::from_str("(listen: \"127.0.0.1:9000\", admin: \"0.0.0.0:8080\")").unwrap();
        assert_eq!(settings.listen.port(), 9000);
        assert!(settings.tunnel == TunnelMode::Quick);
    }

    #[test]
    fn every_tunnel_mode_reads_from_ron() {
        let off: Settings = ron::from_str("(tunnel: Off)").unwrap();
        let domain: Settings = ron::from_str("(tunnel: Domain(token: \"abc\"))").unwrap();
        assert!(off.tunnel == TunnelMode::Off);
        assert!(
            domain.tunnel
                == TunnelMode::Domain {
                    token: "abc".into()
                }
        );
    }
}
