use std::time::Duration;
#[cfg(feature = "standalone")]
use std::{
    env, io,
    process::{Child, Command},
};

use haze::Pack;
use relay_core::{DataDir, Error};
use relay_proxy::{IdleTimeout, Listen};
use relay_storage::FileStore;
use relay_tunnel::{Tunnel, TunnelConfig};

use crate::Settings;

#[derive(Clone, Pack)]
pub struct SettingsStore {
    file: FileStore<Settings>,
    tunnel: Tunnel,
    idle_timeout: IdleTimeout,
}

#[haze::resource]
fn settings_file(dir: DataDir) -> Result<FileStore<Settings>, Error> {
    FileStore::open(dir.0.join("relay.ron"))
}

#[haze::resource]
fn listen(file: FileStore<Settings>) -> Listen {
    Listen(file.read().listen)
}

#[haze::resource]
fn idle_timeout(file: FileStore<Settings>) -> IdleTimeout {
    IdleTimeout::new(file.read().idle_timeout)
}

#[haze::resource]
fn tunnel_config(file: FileStore<Settings>) -> TunnelConfig {
    let settings = file.read();
    TunnelConfig {
        target: format!("http://{}", settings.listen),
        mode: settings.tunnel.clone(),
    }
}

impl SettingsStore {
    const RESTART_DELAY: Duration = Duration::from_millis(300);

    pub fn current(&self) -> Settings {
        self.file.read().clone()
    }

    pub fn save(&self, settings: Settings) -> Result<(), Error> {
        let mode = settings.tunnel.clone();
        let idle_timeout = settings.idle_timeout;
        self.file.set(settings)?;
        self.tunnel.reconfigure(mode);
        self.idle_timeout.set(idle_timeout);
        Ok(())
    }

    pub fn restart(&self) {
        tokio::spawn(async {
            tokio::time::sleep(Self::RESTART_DELAY).await;
            #[cfg(feature = "standalone")]
            if Self::start_next_copy().is_err() {
                return;
            }
            std::process::exit(0);
        });
    }

    #[cfg(feature = "standalone")]
    fn start_next_copy() -> io::Result<Child> {
        Command::new(env::current_exe()?).spawn()
    }
}
