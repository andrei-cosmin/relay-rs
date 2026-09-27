use std::{
    collections::HashMap,
    fs::{self, OpenOptions, Permissions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicU16, Ordering},
    },
};

use tokio::sync::Notify;

use crate::VpnInfo;

struct Vpn {
    port: u16,
    up: bool,
}

pub struct VpnState {
    dir: PathBuf,
    vpns: Mutex<HashMap<String, Vpn>>,
    changed: Notify,
    next_port: AtomicU16,
}

impl VpnState {
    const FIRST_PORT: u16 = 41000;

    pub fn new() -> Self {
        let state = Self { dir: PathBuf::from("data/vpn"), vpns: Mutex::new(HashMap::new()), changed: Notify::new(), next_port: AtomicU16::new(Self::FIRST_PORT) };
        state.load();
        state
    }

    pub fn list(&self) -> Vec<VpnInfo> {
        let mut list = Vec::new();
        for (name, vpn) in self.vpns().iter() {
            list.push(VpnInfo { name: name.clone(), up: vpn.up });
        }
        list.sort_by(|left, right| left.name.cmp(&right.name));
        list
    }

    pub fn add(&self, name: String, config: String) -> Result<(), String> {
        let name = name.trim().to_owned();
        Self::validate(&name, &config)?;
        let mut vpns = self.vpns();
        if vpns.contains_key(&name) {
            return Err(format!("vpn {name} already exists"));
        }
        let port = self.next_port.fetch_add(1, Ordering::Relaxed);
        self.write(&name, &format!("{config}\n[Socks5]\nBindAddress = 127.0.0.1:{port}\n"))?;
        vpns.insert(name, Vpn { port, up: false });
        drop(vpns);
        self.changed.notify_one();
        Ok(())
    }

    pub fn remove(&self, name: String) -> Result<(), String> {
        let mut vpns = self.vpns();
        if !vpns.contains_key(&name) {
            return Err(format!("no vpn named {name}"));
        }
        if let Err(error) = fs::remove_file(self.config_path(&name)) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("vpn {name}: {error}"));
            }
        }
        vpns.remove(&name);
        drop(vpns);
        self.changed.notify_one();
        Ok(())
    }

    pub fn set_up(&self, name: &str, up: bool) {
        if let Some(vpn) = self.vpns().get_mut(name) {
            vpn.up = up;
        }
    }

    pub fn port(&self, name: &str) -> Option<u16> {
        self.vpns().get(name).map(|vpn| vpn.port)
    }

    pub fn names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for name in self.vpns().keys() {
            names.push(name.clone());
        }
        names
    }

    pub fn config_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.conf"))
    }

    pub async fn changed(&self) {
        self.changed.notified().await;
    }

    fn load(&self) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        let mut vpns = self.vpns();
        let mut highest = None;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("conf") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if !Self::valid_name(name) {
                continue;
            }
            let Some(port) = fs::read_to_string(&path).ok().and_then(|text| Self::bound_port(&text)) else {
                println!("vpn {name}: no [Socks5] BindAddress in {}, skipped", path.display());
                continue;
            };
            highest = highest.max(Some(port));
            vpns.insert(name.to_owned(), Vpn { port, up: false });
        }
        if let Some(port) = highest {
            self.next_port.store(Self::FIRST_PORT.max(port.saturating_add(1)), Ordering::Relaxed);
        }
    }

    fn write(&self, name: &str, text: &str) -> Result<(), String> {
        let path = self.config_path(name);
        let fail = |error: std::io::Error| format!("{}: {error}", path.display());
        fs::create_dir_all(&self.dir).map_err(fail)?;
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&path).map_err(fail)?;
        file.set_permissions(Permissions::from_mode(0o600)).map_err(fail)?;
        file.write_all(text.as_bytes()).map_err(fail)
    }

    fn validate(name: &str, config: &str) -> Result<(), String> {
        if !Self::valid_name(name) {
            return Err("name must be 1 to 128 letters, digits, spaces or dashes".to_owned());
        }
        for required in ["[Interface]", "[Peer]", "PrivateKey"] {
            if !config.contains(required) {
                return Err(format!("config is missing {required}"));
            }
        }
        if config.contains("[Socks5]") {
            return Err("config must not contain a [Socks5] section, relay adds its own".to_owned());
        }
        Ok(())
    }

    fn valid_name(name: &str) -> bool {
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

    fn bound_port(text: &str) -> Option<u16> {
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
                return value.trim().rsplit_once(':').and_then(|(_, port)| port.parse().ok());
            }
        }
        None
    }

    fn vpns(&self) -> MutexGuard<'_, HashMap<String, Vpn>> {
        self.vpns.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "[Interface]\nPrivateKey = abc\nAddress = 10.0.0.2/32\n\n[Peer]\nPublicKey = def\nEndpoint = 1.2.3.4:51820\n";

    fn state(name: &str) -> VpnState {
        let dir = std::env::temp_dir().join(format!("relay-vpn-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        VpnState { dir, vpns: Mutex::new(HashMap::new()), changed: Notify::new(), next_port: AtomicU16::new(VpnState::FIRST_PORT) }
    }

    fn reopened(state: &VpnState) -> VpnState {
        let again = VpnState { dir: state.dir.clone(), vpns: Mutex::new(HashMap::new()), changed: Notify::new(), next_port: AtomicU16::new(VpnState::FIRST_PORT) };
        again.load();
        again
    }

    #[test]
    fn names_are_letters_digits_spaces_and_dashes_up_to_128() {
        assert!(VpnState::valid_name("Warp 1-a"));
        assert!(VpnState::valid_name(&"x".repeat(128)));
        assert!(!VpnState::valid_name(""));
        assert!(!VpnState::valid_name("a_b"));
        assert!(!VpnState::valid_name(&"x".repeat(129)));
    }

    #[test]
    fn configs_need_interface_peer_and_key_but_no_socks() {
        assert!(VpnState::validate("a", CONFIG).is_ok());
        assert_eq!(VpnState::validate("a", "[Interface]\nPrivateKey = x\n").unwrap_err(), "config is missing [Peer]");
        assert!(VpnState::validate("a", &format!("{CONFIG}[Socks5]\n")).unwrap_err().contains("[Socks5]"));
        assert!(VpnState::validate("a_b", CONFIG).unwrap_err().starts_with("name must be"));
    }

    #[test]
    fn add_writes_a_private_file_with_its_own_socks_port() {
        let state = state("add");
        state.add("Warp".into(), CONFIG.into()).unwrap();
        state.add("Own VPS".into(), CONFIG.into()).unwrap();
        assert_eq!((state.port("Warp"), state.port("Own VPS")), (Some(41000), Some(41001)));
        let text = fs::read_to_string(state.config_path("Warp")).unwrap();
        assert!(text.ends_with("[Socks5]\nBindAddress = 127.0.0.1:41000\n"));
        assert_eq!(fs::metadata(state.config_path("Warp")).unwrap().permissions().mode() & 0o777, 0o600);
        assert!(state.add("Warp".into(), CONFIG.into()).unwrap_err().contains("already exists"));
        assert_eq!(state.list().iter().map(|vpn| vpn.name.as_str()).collect::<Vec<_>>(), ["Own VPS", "Warp"]);
        let _ = fs::remove_dir_all(&state.dir);
    }

    #[test]
    fn remove_deletes_the_file() {
        let state = state("remove");
        state.add("Warp".into(), CONFIG.into()).unwrap();
        state.remove("Warp".into()).unwrap();
        assert!(state.port("Warp").is_none() && !state.config_path("Warp").exists());
        assert!(state.remove("Warp".into()).unwrap_err().starts_with("no vpn named"));
        let _ = fs::remove_dir_all(&state.dir);
    }

    #[test]
    fn load_reads_the_files_back_and_continues_the_ports() {
        let state = state("load");
        state.add("Warp".into(), CONFIG.into()).unwrap();
        state.add("Own VPS".into(), CONFIG.into()).unwrap();
        let again = reopened(&state);
        assert_eq!(again.port("Own VPS"), Some(41001));
        again.add("Third".into(), CONFIG.into()).unwrap();
        assert_eq!(again.port("Third"), Some(41002));
        let _ = fs::remove_dir_all(&state.dir);
    }

    #[test]
    fn bound_port_reads_only_the_socks_section() {
        assert_eq!(VpnState::bound_port("[Interface]\nBindAddress = 1:9\n[Socks5]\nBindAddress = 127.0.0.1:41005\n"), Some(41005));
        assert_eq!(VpnState::bound_port(CONFIG), None);
    }
}
