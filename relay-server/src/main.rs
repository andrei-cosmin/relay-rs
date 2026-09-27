#[cfg(feature = "server")]
mod manager;

#[cfg(feature = "web")]
fn main() {
    dioxus::launch(relay_ui::App);
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> Result<(), relay_core::Error> {
    use manager::Manager;
    use relay_admin::{Admin, Serve};
    use relay_core::{Config, Egress, Error, Resources, Sequence, TargetCache};
    use relay_monitor::{Entry, EntryLog, Monitor, MonitorController};
    use relay_proxy::{Listen, Proxy};
    use relay_settings::{Settings, SettingsController};
    use relay_storage::Storage;
    use relay_targets::{Rules, TargetsController};
    use relay_tunnel::{Tunnel, TunnelController, TunnelState};
    use relay_vpn::{VpnController, VpnState, Vpns};

    rustls_graviola::default_provider().install_default().map_err(|_| Error::internal("a tls provider is already installed"))?;
    let settings = Settings::load()?;
    let mut resources = Resources::new();
    resources.insert(TargetCache::load()?);
    resources.insert(Listen(settings.listen));
    resources.insert(Serve(settings.admin));
    resources.insert(TunnelState::new(format!("http://{}", settings.listen), settings.tunnel.clone()));
    resources.insert(Storage::open("data/relay.redb")?);
    resources.insert(Sequence::after(resources.get::<Storage>().newest_key::<Entry>()?));
    resources.insert(EntryLog::new(resources.get()));
    resources.insert(Egress::new());
    resources.insert(VpnState::new());
    resources.insert(settings);
    let mut manager = Manager::new(resources);
    manager.leaf::<Monitor>();
    manager.leaf::<Rules>();
    manager.controller::<TargetsController>();
    manager.controller::<TunnelController>();
    manager.controller::<MonitorController>();
    manager.controller::<SettingsController>();
    manager.controller::<VpnController>();
    manager.wire();
    manager.service::<Tunnel>();
    manager.service::<Proxy>();
    manager.service::<Vpns>();
    manager.service::<Admin>();
    manager.run().await
}
