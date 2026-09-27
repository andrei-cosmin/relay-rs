use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use relay_core::{Egress, Error, Resources, Service};
use tokio::process::{Child, Command};

use crate::VpnState;

pub struct Vpns {
    state: Arc<VpnState>,
    egress: Arc<Egress>,
}

impl Vpns {
    const CHECK_EVERY: Duration = Duration::from_secs(5);
    const RESPAWN_AFTER: Duration = Duration::from_secs(2);

    async fn reconcile(&self, children: &mut HashMap<String, Child>) {
        let names = self.state.names();
        let mut removed = Vec::new();
        for name in children.keys() {
            if !names.contains(name) {
                removed.push(name.clone());
            }
        }
        for name in removed {
            if let Some(mut child) = children.remove(&name) {
                let _ = child.kill().await;
            }
            self.egress.remove(&name);
            println!("vpn {name}: removed");
        }
        for name in names {
            if children.contains_key(&name) {
                continue;
            }
            let Some(port) = self.state.port(&name) else {
                continue;
            };
            if let Some(child) = self.spawn(&name, port) {
                children.insert(name, child);
            }
        }
    }

    fn spawn(&self, name: &str, port: u16) -> Option<Child> {
        let spawned = Command::new("wireproxy")
            .arg("-c")
            .arg(self.state.config_path(name))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn();
        match spawned {
            Ok(child) => {
                let socks = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
                println!("vpn {name}: wireproxy started, socks5 on {socks}");
                self.state.set_up(name, true);
                self.egress.set(name, socks);
                Some(child)
            }
            Err(error) => {
                println!("vpn {name}: wireproxy failed to start: {error}");
                self.state.set_up(name, false);
                None
            }
        }
    }

    async fn check(&self, children: &mut HashMap<String, Child>) {
        let mut exited = Vec::new();
        for (name, child) in children.iter_mut() {
            match child.try_wait() {
                Ok(None) => {}
                Ok(Some(status)) => {
                    println!("vpn {name}: wireproxy exited ({status})");
                    exited.push(name.clone());
                }
                Err(error) => {
                    println!("vpn {name}: wireproxy lost: {error}");
                    exited.push(name.clone());
                }
            }
        }
        if exited.is_empty() {
            return;
        }
        for name in exited {
            children.remove(&name);
            self.state.set_up(&name, false);
            self.egress.remove(&name);
        }
        tokio::time::sleep(Self::RESPAWN_AFTER).await;
    }
}

#[async_trait::async_trait]
impl Service for Vpns {
    fn build(resources: &Resources) -> Self {
        Self { state: resources.get::<VpnState>(), egress: resources.get::<Egress>() }
    }

    async fn run(&self) -> Result<(), Error> {
        let supervise = async {
            let mut children = HashMap::new();
            let mut tick = tokio::time::interval(Self::CHECK_EVERY);
            loop {
                self.reconcile(&mut children).await;
                tokio::select! {
                    _ = self.state.changed() => {}
                    _ = tick.tick() => self.check(&mut children).await,
                }
            }
        };
        tokio::select! {
            _ = supervise => Ok(()),
            _ = Self::until_shutdown() => Ok(()),
        }
    }
}
