use std::{process::Stdio, sync::Arc, time::Duration};

use relay_core::{Error, Resources, Service, TunnelMode};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

use crate::TunnelState;

pub struct Tunnel {
    state: Arc<TunnelState>,
}

impl Tunnel {
    fn command(mode: &TunnelMode, target: &str) -> Option<Command> {
        let mut command = Command::new("cloudflared");
        match mode {
            TunnelMode::Off => return None,
            TunnelMode::Quick => command.args(["tunnel", "--no-autoupdate", "--url", target]),
            TunnelMode::Domain { token } => command.args(["tunnel", "--no-autoupdate", "run"]).env("TUNNEL_TOKEN", token),
        };
        command.stdout(Stdio::null()).stderr(Stdio::piped()).kill_on_drop(true);
        Some(command)
    }

    fn spawn(&self) -> Result<Child, Error> {
        let Some(mut command) = Self::command(self.state.mode(), self.state.target()) else {
            return Err(Error::internal("the tunnel is off"));
        };
        command.spawn().map_err(|error| Error::internal(format!("cloudflared: {error}")))
    }

    async fn attend(&self, child: &mut Child) {
        if let Some(stderr) = child.stderr.take() {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(url) = Self::url_in(&line) {
                    if url != self.state.url() {
                        println!("tunnel: {url}");
                        self.state.set_url(url);
                    }
                }
            }
        }
        let _ = child.wait().await;
        self.state.clear();
    }

    fn url_in(line: &str) -> Option<String> {
        for word in line.split_whitespace() {
            if word.starts_with("https://") && word.ends_with(".trycloudflare.com") {
                return Some(word.to_owned());
            }
        }
        let unescaped = line.replace('\\', "");
        let (_, rest) = unescaped.split_once("\"hostname\":\"")?;
        let (host, _) = rest.split_once('"')?;
        (!host.is_empty()).then(|| format!("https://{host}"))
    }
}

#[async_trait::async_trait]
impl Service for Tunnel {
    fn build(resources: &Resources) -> Self {
        Self { state: resources.get::<TunnelState>() }
    }

    async fn run(&self) -> Result<(), Error> {
        if !self.state.mode().enabled() {
            Self::until_shutdown().await;
            return Ok(());
        }
        let mut child = Some(self.spawn()?);
        let supervise = async {
            loop {
                if let Some(running) = child.as_mut() {
                    let crashed = tokio::select! {
                        _ = self.attend(running) => true,
                        _ = self.state.wait_for_restart() => false,
                    };
                    if crashed {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    } else {
                        let _ = running.kill().await;
                        self.state.clear();
                    }
                }
                child = self.spawn().ok();
            }
        };
        tokio::select! {
            _ = supervise => Ok(()),
            _ = Self::until_shutdown() => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn arguments(command: &Command) -> Vec<&OsStr> {
        command.as_std().get_args().collect()
    }

    #[test]
    fn the_quick_tunnel_url_is_found_in_cloudflared_output() {
        let line = "2026-09-28T00:00:00Z INF |  https://font-same-incredible-unknown.trycloudflare.com  |";
        assert_eq!(Tunnel::url_in(line).as_deref(), Some("https://font-same-incredible-unknown.trycloudflare.com"));
        assert_eq!(Tunnel::url_in("INF Requesting new quick Tunnel on trycloudflare.com..."), None);
        assert_eq!(Tunnel::url_in("https://api.trycloudflare.com/tunnel"), None);
    }

    #[test]
    fn the_own_domain_is_found_in_the_pushed_configuration() {
        let line = r#"INF Updated to new configuration config="{\"ingress\":[{\"hostname\":\"relay.example.com\",\"service\":\"http://localhost:8585\"},{\"service\":\"http_status:404\"}]}" version=1"#;
        assert_eq!(Tunnel::url_in(line).as_deref(), Some("https://relay.example.com"));
        assert_eq!(Tunnel::url_in(r#"INF Registered tunnel connection connIndex=0"#), None);
    }

    #[test]
    fn each_mode_runs_its_own_command() {
        assert!(Tunnel::command(&TunnelMode::Off, "http://127.0.0.1:8585").is_none());
        let quick = Tunnel::command(&TunnelMode::Quick, "http://127.0.0.1:8585").unwrap();
        assert_eq!(arguments(&quick), ["tunnel", "--no-autoupdate", "--url", "http://127.0.0.1:8585"]);
        let domain = Tunnel::command(&TunnelMode::Domain { token: "secret".into() }, "http://127.0.0.1:8585").unwrap();
        assert_eq!(arguments(&domain), ["tunnel", "--no-autoupdate", "run"]);
        let token = domain.as_std().get_envs().find(|(name, _)| *name == "TUNNEL_TOKEN").and_then(|(_, value)| value);
        assert_eq!(token, Some(OsStr::new("secret")));
    }
}
