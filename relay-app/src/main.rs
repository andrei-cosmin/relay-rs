#[cfg(feature = "web")]
fn main() {
    dioxus::launch(relay_ui::App);
}

#[cfg(feature = "server")]
fn main() {
    use std::path::PathBuf;

    use anyhow::anyhow;
    use relay_core::DataDir;
    use {
        relay_monitor as _, relay_proxy as _, relay_settings as _, relay_targets as _,
        relay_tunnel as _, relay_vpn as _,
    };

    haze::serve(
        async |resources| {
            rustls_graviola::default_provider()
                .install_default()
                .map_err(|_| anyhow!("a tls provider is already installed"))?;
            resources.insert(DataDir(PathBuf::from("data")));
            Ok(())
        },
        haze::client_rendered,
    );
}

#[cfg(feature = "standalone")]
fn main() {
    use anyhow::{Context, anyhow};
    use relay_core::DataDir;
    use relay_ui::TitleBarWindow;
    use {
        relay_monitor as _, relay_proxy as _, relay_settings as _, relay_targets as _,
        relay_tunnel as _, relay_vpn as _,
    };

    dioxus::logger::initialize_default();
    let _resources = haze::install(async |resources| {
        rustls_graviola::default_provider()
            .install_default()
            .map_err(|_| anyhow!("a tls provider is already installed"))?;
        let data = dirs::data_dir().context("this system has no data folder")?;
        resources.insert(DataDir(data.join("Relay-RS")));
        Ok(())
    });
    dioxus::LaunchBuilder::desktop()
        .with_cfg(TitleBarWindow::config(
            "Relay-RS",
            (1280.0, 840.0),
            (980.0, 640.0),
        ))
        .launch(relay_ui::App);
}
