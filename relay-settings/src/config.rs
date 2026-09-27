use relay_core::Config;

use crate::Settings;

impl Config for Settings {
    const FILE: &'static str = "relay.ron";
}

#[cfg(test)]
mod tests {
    use relay_core::TunnelMode;

    use super::*;

    #[test]
    fn defaults_are_local_proxy_lan_page_and_a_quick_tunnel() {
        let settings = Settings::default();
        assert_eq!(settings.listen.to_string(), "127.0.0.1:8585");
        assert_eq!(settings.admin.to_string(), "0.0.0.0:8080");
        assert!(settings.tunnel == TunnelMode::Quick);
    }

    #[test]
    fn an_old_file_without_a_tunnel_gets_the_quick_one() {
        let settings: Settings = ron::from_str("(listen: \"127.0.0.1:9000\", admin: \"0.0.0.0:8080\")").unwrap();
        assert_eq!(settings.listen.port(), 9000);
        assert!(settings.tunnel == TunnelMode::Quick);
    }

    #[test]
    fn every_tunnel_mode_reads_from_ron() {
        let off: Settings = ron::from_str("(tunnel: Off)").unwrap();
        let domain: Settings = ron::from_str("(tunnel: Domain(token: \"abc\"))").unwrap();
        assert!(off.tunnel == TunnelMode::Off);
        assert!(domain.tunnel == TunnelMode::Domain { token: "abc".into() });
    }

    #[test]
    fn the_file_lives_in_data() {
        assert_eq!(Settings::path(), "data/relay.ron");
    }
}
