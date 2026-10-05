use std::{
    process::Stdio,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use futures_util::{Stream, stream};
use relay_core::Tool;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{Notify, watch},
};

use crate::{TunnelConfig, TunnelMode};

pub(crate) struct Supervisor {
    setup: Mutex<(String, TunnelMode)>,
    url_sender: watch::Sender<String>,
    restart_requested: Notify,
}

impl Supervisor {
    const RETRY_DELAY: Duration = Duration::from_secs(2);

    pub(crate) fn spawn(config: &TunnelConfig) -> Arc<Self> {
        let supervisor = Arc::new(Self {
            setup: Mutex::new((config.target.clone(), config.mode.clone())),
            url_sender: watch::channel(String::new()).0,
            restart_requested: Notify::new(),
        });
        tokio::spawn(Arc::clone(&supervisor).supervise());
        supervisor
    }

    pub(crate) fn current_url(&self) -> String {
        self.url_sender.borrow().clone()
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.lock_setup().1.enabled()
    }

    pub(crate) fn request_restart(&self) {
        self.restart_requested.notify_one();
    }

    pub(crate) fn reconfigure(&self, target: String, mode: TunnelMode) {
        *self.lock_setup() = (target, mode);
        self.request_restart();
    }

    pub(crate) fn url_updates(&self) -> impl Stream<Item = String> + Send + 'static {
        stream::unfold(self.url_sender.subscribe(), |mut receiver| async move {
            receiver.changed().await.ok()?;
            let url = receiver.borrow_and_update().clone();
            Some((url, receiver))
        })
    }

    async fn supervise(self: Arc<Self>) {
        loop {
            let Some(mut command) = self.cloudflared_command() else {
                self.restart_requested.notified().await;
                continue;
            };
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(error) => {
                    tracing::error!("tunnel: cloudflared: {error}");
                    self.wait_before_retry().await;
                    continue;
                }
            };
            let restarted = tokio::select! {
                () = self.publish_url_until_exit(&mut child) => false,
                () = self.restart_requested.notified() => true,
            };
            if restarted {
                let _ = child.kill().await;
            } else {
                self.wait_before_retry().await;
            }
            self.url_sender.send_replace(String::new());
        }
    }

    fn cloudflared_command(&self) -> Option<Command> {
        let (target, mode) = self.lock_setup().clone();
        let mut command = Command::new(Tool("cloudflared").program());
        match mode {
            TunnelMode::Off => return None,
            TunnelMode::Quick => command.args(["tunnel", "--no-autoupdate", "--url", &target]),
            TunnelMode::Domain { token } => command
                .args(["tunnel", "--no-autoupdate", "run"])
                .env("TUNNEL_TOKEN", token),
        };
        command
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        Some(command)
    }

    async fn publish_url_until_exit(&self, child: &mut Child) {
        if let Some(stderr) = child.stderr.take() {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(url) = Self::url_from_log_line(&line)
                    && url != self.current_url()
                {
                    tracing::info!("tunnel: {url}");
                    self.url_sender.send_replace(url);
                }
            }
        }
        let _ = child.wait().await;
    }

    async fn wait_before_retry(&self) {
        tokio::select! {
            () = tokio::time::sleep(Self::RETRY_DELAY) => {}
            () = self.restart_requested.notified() => {}
        }
    }

    fn url_from_log_line(line: &str) -> Option<String> {
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

    fn lock_setup(&self) -> MutexGuard<'_, (String, TunnelMode)> {
        self.setup
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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
        let line =
            "2026-09-28T00:00:00Z INF |  https://font-same-incredible-unknown.trycloudflare.com  |";
        assert_eq!(
            Supervisor::url_from_log_line(line).as_deref(),
            Some("https://font-same-incredible-unknown.trycloudflare.com")
        );
        assert_eq!(
            Supervisor::url_from_log_line(
                "INF Requesting new quick Tunnel on trycloudflare.com..."
            ),
            None
        );
        assert_eq!(
            Supervisor::url_from_log_line("https://api.trycloudflare.com/tunnel"),
            None
        );
    }

    #[test]
    fn the_own_domain_is_found_in_the_pushed_configuration() {
        let line = r#"INF Updated to new configuration config="{\"ingress\":[{\"hostname\":\"relay.example.com\",\"service\":\"http://localhost:8585\"},{\"service\":\"http_status:404\"}]}" version=1"#;
        assert_eq!(
            Supervisor::url_from_log_line(line).as_deref(),
            Some("https://relay.example.com")
        );
        assert_eq!(
            Supervisor::url_from_log_line(r#"INF Registered tunnel connection connIndex=0"#),
            None
        );
    }

    fn configured(mode: TunnelMode) -> Supervisor {
        Supervisor {
            setup: Mutex::new((String::from("http://127.0.0.1:8585"), mode)),
            url_sender: watch::channel(String::new()).0,
            restart_requested: Notify::new(),
        }
    }

    fn command(mode: TunnelMode) -> Option<Command> {
        configured(mode).cloudflared_command()
    }

    #[test]
    fn each_mode_runs_its_own_command() {
        assert!(command(TunnelMode::Off).is_none());
        let quick = command(TunnelMode::Quick).unwrap();
        assert_eq!(
            arguments(&quick),
            [
                "tunnel",
                "--no-autoupdate",
                "--url",
                "http://127.0.0.1:8585"
            ]
        );
        let domain = command(TunnelMode::Domain {
            token: "secret".into(),
        })
        .unwrap();
        assert_eq!(arguments(&domain), ["tunnel", "--no-autoupdate", "run"]);
        let token = domain
            .as_std()
            .get_envs()
            .find(|(name, _)| *name == "TUNNEL_TOKEN")
            .and_then(|(_, value)| value);
        assert_eq!(token, Some(OsStr::new("secret")));
    }

    #[test]
    fn reconfigure_switches_the_mode_and_the_target() {
        let supervisor = configured(TunnelMode::Off);
        assert!(!supervisor.is_enabled());
        supervisor.reconfigure(String::from("http://127.0.0.1:9000"), TunnelMode::Quick);
        assert!(supervisor.is_enabled());
        let quick = supervisor.cloudflared_command().unwrap();
        assert_eq!(
            arguments(&quick),
            [
                "tunnel",
                "--no-autoupdate",
                "--url",
                "http://127.0.0.1:9000"
            ]
        );
    }
}
