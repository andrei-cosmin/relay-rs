use std::net::SocketAddr;

use relay_settings::Settings;
use relay_tunnel::TunnelMode;

#[derive(Clone, PartialEq)]
pub(crate) struct SettingsDraft {
    pub(crate) listen: String,
    pub(crate) tunnel: String,
    pub(crate) token: String,
}

impl SettingsDraft {
    pub(crate) const OFF: &'static str = "Off";
    pub(crate) const QUICK: &'static str = "Quick";
    pub(crate) const DOMAIN: &'static str = "Own domain";

    pub(super) fn from_settings(settings: Settings) -> Self {
        let (tunnel, token) = match settings.tunnel {
            TunnelMode::Off => (String::new(), String::new()),
            TunnelMode::Quick => (Self::QUICK.to_owned(), String::new()),
            TunnelMode::Domain { token } => (Self::DOMAIN.to_owned(), token),
        };
        Self {
            listen: settings.listen.to_string(),
            tunnel,
            token,
        }
    }

    pub(super) fn parse(&self) -> Result<Settings, String> {
        Ok(Settings {
            listen: Self::address("listen", &self.listen)?,
            tunnel: self.tunnel_mode()?,
        })
    }

    pub(crate) fn wants_token(&self) -> bool {
        self.tunnel == Self::DOMAIN
    }

    fn tunnel_mode(&self) -> Result<TunnelMode, String> {
        match self.tunnel.as_str() {
            "" => Ok(TunnelMode::Off),
            Self::QUICK => Ok(TunnelMode::Quick),
            Self::DOMAIN if self.token.trim().is_empty() => {
                Err("tunnel: own domain needs the tunnel token".to_owned())
            }
            Self::DOMAIN => Ok(TunnelMode::Domain {
                token: self.token.trim().to_owned(),
            }),
            other => Err(format!("tunnel: unknown mode {other}")),
        }
    }

    fn address(name: &str, text: &str) -> Result<SocketAddr, String> {
        text.trim()
            .parse()
            .map_err(|_| format!("{name}: invalid socket address"))
    }
}
