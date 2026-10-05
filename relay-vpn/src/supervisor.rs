use std::{
    collections::HashMap,
    fs::{self, OpenOptions, Permissions},
    io::Write,
    net::{Ipv4Addr, SocketAddr},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU16, Ordering},
    },
    time::Duration,
};

use relay_core::Tool;
use tokio::{
    process::{Child, Command},
    sync::Notify,
};

use crate::{Vpn, VpnConfig, VpnInfo};

pub(crate) struct Supervisor {
    dir: PathBuf,
    vpns: Mutex<HashMap<String, Vpn>>,
    list_changed: Notify,
    next_port: AtomicU16,
}

impl Supervisor {
    const FIRST_PORT: u16 = 41000;
    const CHECK_INTERVAL: Duration = Duration::from_secs(5);
    const RESTART_DELAY: Duration = Duration::from_secs(2);

    pub(crate) fn spawn(config: &VpnConfig) -> Arc<Self> {
        let supervisor = Arc::new(Self {
            dir: config.dir.clone(),
            vpns: Mutex::new(HashMap::new()),
            list_changed: Notify::new(),
            next_port: AtomicU16::new(Self::FIRST_PORT),
        });
        supervisor.load_configs();
        tokio::spawn(Arc::clone(&supervisor).supervise());
        supervisor
    }

    pub(crate) fn list(&self) -> Vec<VpnInfo> {
        let mut list = Vec::new();
        for (name, vpn) in self.lock_vpns().iter() {
            list.push(VpnInfo {
                name: name.clone(),
                up: vpn.up,
            });
        }
        list.sort_by(|left, right| left.name.cmp(&right.name));
        list
    }

    pub(crate) fn add(&self, name: String, config: String) -> Result<(), String> {
        let name = name.trim().to_owned();
        Self::validate_config(&name, &config)?;
        let mut vpns = self.lock_vpns();
        if vpns.contains_key(&name) {
            return Err(format!("vpn {name} already exists"));
        }
        let port = self.next_port.fetch_add(1, Ordering::Relaxed);
        self.write_config(
            &name,
            &format!("{config}\n[Socks5]\nBindAddress = 127.0.0.1:{port}\n"),
        )?;
        vpns.insert(name, Vpn { port, up: false });
        drop(vpns);
        self.list_changed.notify_one();
        Ok(())
    }

    pub(crate) fn remove(&self, name: String) -> Result<(), String> {
        let mut vpns = self.lock_vpns();
        if !vpns.contains_key(&name) {
            return Err(format!("no vpn named {name}"));
        }
        if let Err(error) = fs::remove_file(self.config_path(&name))
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(format!("vpn {name}: {error}"));
        }
        vpns.remove(&name);
        drop(vpns);
        self.list_changed.notify_one();
        Ok(())
    }

    pub(crate) fn socks_address(&self, name: &str) -> Option<SocketAddr> {
        let vpns = self.lock_vpns();
        let vpn = vpns.get(name).filter(|vpn| vpn.up)?;
        Some(SocketAddr::from((Ipv4Addr::LOCALHOST, vpn.port)))
    }

    fn mark_running(&self, name: &str, up: bool) {
        if let Some(vpn) = self.lock_vpns().get_mut(name) {
            vpn.up = up;
        }
    }

    fn ports(&self) -> HashMap<String, u16> {
        let mut ports = HashMap::new();
        for (name, vpn) in self.lock_vpns().iter() {
            ports.insert(name.clone(), vpn.port);
        }
        ports
    }

    fn config_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.conf"))
    }

    async fn supervise(self: Arc<Self>) {
        let mut processes = HashMap::new();
        let mut ticker = tokio::time::interval(Self::CHECK_INTERVAL);
        loop {
            self.sync_processes(&mut processes);
            tokio::select! {
                () = self.list_changed.notified() => {}
                _ = ticker.tick() => self.drop_exited(&mut processes).await,
            }
        }
    }

    fn sync_processes(&self, processes: &mut HashMap<String, (u16, Child)>) {
        let ports = self.ports();
        processes.retain(|name, (port, _)| ports.get(name) == Some(port));
        for (name, port) in ports {
            if processes.contains_key(&name) {
                continue;
            }
            if let Some(child) = self.start_wireproxy(&name, port) {
                processes.insert(name, (port, child));
            }
        }
    }

    fn start_wireproxy(&self, name: &str, port: u16) -> Option<Child> {
        let started = Command::new(Tool("wireproxy").program())
            .arg("-c")
            .arg(self.config_path(name))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn();
        match started {
            Ok(child) => {
                tracing::info!("vpn {name}: wireproxy started, socks5 on 127.0.0.1:{port}");
                self.mark_running(name, true);
                Some(child)
            }
            Err(error) => {
                tracing::error!("vpn {name}: wireproxy failed to start: {error}");
                self.mark_running(name, false);
                None
            }
        }
    }

    async fn drop_exited(&self, processes: &mut HashMap<String, (u16, Child)>) {
        let before = processes.len();
        processes.retain(|name, (_, child)| {
            let exited = match child.try_wait() {
                Ok(None) => return true,
                Ok(Some(status)) => format!("exited ({status})"),
                Err(error) => format!("lost: {error}"),
            };
            tracing::warn!("vpn {name}: wireproxy {exited}");
            self.mark_running(name, false);
            false
        });
        if processes.len() < before {
            tokio::time::sleep(Self::RESTART_DELAY).await;
        }
    }

    fn load_configs(&self) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        let mut vpns = self.lock_vpns();
        let mut highest = None;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("conf") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if !Self::is_valid_name(name) {
                continue;
            }
            let Some(port) = fs::read_to_string(&path)
                .ok()
                .and_then(|text| Self::socks_port_in_config(&text))
            else {
                tracing::warn!(
                    "vpn {name}: no [Socks5] BindAddress in {}, skipped",
                    path.display()
                );
                continue;
            };
            highest = highest.max(Some(port));
            vpns.insert(name.to_owned(), Vpn { port, up: false });
        }
        if let Some(port) = highest {
            self.next_port.store(
                Self::FIRST_PORT.max(port.saturating_add(1)),
                Ordering::Relaxed,
            );
        }
    }

    fn write_config(&self, name: &str, text: &str) -> Result<(), String> {
        let path = self.config_path(name);
        let fail = |error: std::io::Error| format!("{}: {error}", path.display());
        fs::create_dir_all(&self.dir).map_err(fail)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(fail)?;
        file.set_permissions(Permissions::from_mode(0o600))
            .map_err(fail)?;
        file.write_all(text.as_bytes()).map_err(fail)
    }

    fn validate_config(name: &str, config: &str) -> Result<(), String> {
        if !Self::is_valid_name(name) {
            return Err("name must be 1 to 128 letters, digits, spaces or dashes".to_owned());
        }
        for required in ["[Interface]", "[Peer]", "PrivateKey"] {
            if !config.contains(required) {
                return Err(format!("config is missing {required}"));
            }
        }
        if config.contains("[Socks5]") {
            return Err(
                "config must not contain a [Socks5] section, relay adds its own".to_owned(),
            );
        }
        Ok(())
    }

    fn is_valid_name(name: &str) -> bool {
        if name.is_empty() || name.len() > 128 {
            return false;
        }
        for character in name.chars() {
            if !(character.is_ascii_alphanumeric() || character == ' ' || character == '-') {
                return false;
            }
        }
        true
    }

    fn socks_port_in_config(text: &str) -> Option<u16> {
        let mut in_socks = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_socks = line == "[Socks5]";
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if in_socks && key.trim() == "BindAddress" {
                return value
                    .trim()
                    .rsplit_once(':')
                    .and_then(|(_, port)| port.parse().ok());
            }
        }
        None
    }

    fn lock_vpns(&self) -> MutexGuard<'_, HashMap<String, Vpn>> {
        self.vpns
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "[Interface]\nPrivateKey = abc\nAddress = 10.0.0.2/32\n\n[Peer]\nPublicKey = def\nEndpoint = 1.2.3.4:51820\n";

    fn state(name: &str) -> Supervisor {
        let dir = std::env::temp_dir().join(format!("relay-vpn-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Supervisor {
            dir,
            vpns: Mutex::new(HashMap::new()),
            list_changed: Notify::new(),
            next_port: AtomicU16::new(Supervisor::FIRST_PORT),
        }
    }

    fn reopened(state: &Supervisor) -> Supervisor {
        let again = Supervisor {
            dir: state.dir.clone(),
            vpns: Mutex::new(HashMap::new()),
            list_changed: Notify::new(),
            next_port: AtomicU16::new(Supervisor::FIRST_PORT),
        };
        again.load_configs();
        again
    }

    #[test]
    fn names_are_letters_digits_spaces_and_dashes_up_to_128() {
        assert!(Supervisor::is_valid_name("Warp 1-a"));
        assert!(Supervisor::is_valid_name(&"x".repeat(128)));
        assert!(!Supervisor::is_valid_name(""));
        assert!(!Supervisor::is_valid_name("a_b"));
        assert!(!Supervisor::is_valid_name(&"x".repeat(129)));
    }

    #[test]
    fn configs_need_interface_peer_and_key_but_no_socks() {
        assert!(Supervisor::validate_config("a", CONFIG).is_ok());
        assert_eq!(
            Supervisor::validate_config("a", "[Interface]\nPrivateKey = x\n").unwrap_err(),
            "config is missing [Peer]"
        );
        assert!(
            Supervisor::validate_config("a", &format!("{CONFIG}[Socks5]\n"))
                .unwrap_err()
                .contains("[Socks5]")
        );
        assert!(
            Supervisor::validate_config("a_b", CONFIG)
                .unwrap_err()
                .starts_with("name must be")
        );
    }

    #[test]
    fn add_writes_a_private_file_with_its_own_socks_port() {
        let state = state("add");
        state.add("Warp".into(), CONFIG.into()).unwrap();
        state.add("Own VPS".into(), CONFIG.into()).unwrap();
        assert_eq!(
            (state.ports()["Warp"], state.ports()["Own VPS"]),
            (41000, 41001)
        );
        let text = fs::read_to_string(state.config_path("Warp")).unwrap();
        assert!(text.ends_with("[Socks5]\nBindAddress = 127.0.0.1:41000\n"));
        assert_eq!(
            fs::metadata(state.config_path("Warp"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(
            state
                .add("Warp".into(), CONFIG.into())
                .unwrap_err()
                .contains("already exists")
        );
        assert_eq!(
            state
                .list()
                .iter()
                .map(|vpn| vpn.name.as_str())
                .collect::<Vec<_>>(),
            ["Own VPS", "Warp"]
        );
        let _ = fs::remove_dir_all(&state.dir);
    }

    #[test]
    fn remove_deletes_the_file() {
        let state = state("remove");
        state.add("Warp".into(), CONFIG.into()).unwrap();
        state.remove("Warp".into()).unwrap();
        assert!(state.ports().is_empty() && !state.config_path("Warp").exists());
        assert!(
            state
                .remove("Warp".into())
                .unwrap_err()
                .starts_with("no vpn named")
        );
        let _ = fs::remove_dir_all(&state.dir);
    }

    #[test]
    fn load_reads_the_files_back_and_continues_the_ports() {
        let state = state("load");
        state.add("Warp".into(), CONFIG.into()).unwrap();
        state.add("Own VPS".into(), CONFIG.into()).unwrap();
        let again = reopened(&state);
        assert_eq!(again.ports()["Own VPS"], 41001);
        again.add("Third".into(), CONFIG.into()).unwrap();
        assert_eq!(again.ports()["Third"], 41002);
        let _ = fs::remove_dir_all(&state.dir);
    }

    #[test]
    fn socks_port_in_config_reads_only_the_socks_section() {
        assert_eq!(
            Supervisor::socks_port_in_config(
                "[Interface]\nBindAddress = 1:9\n[Socks5]\nBindAddress = 127.0.0.1:41005\n"
            ),
            Some(41005)
        );
        assert_eq!(Supervisor::socks_port_in_config(CONFIG), None);
    }

    #[test]
    fn a_socks_address_exists_only_while_its_vpn_is_up() {
        let state = state("exit");
        state.add("Warp".into(), CONFIG.into()).unwrap();
        assert_eq!(state.socks_address("Warp"), None);
        state.mark_running("Warp", true);
        assert_eq!(
            state.socks_address("Warp"),
            Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 41000)))
        );
        assert_eq!(state.socks_address("Nope"), None);
        let _ = fs::remove_dir_all(&state.dir);
    }
}
