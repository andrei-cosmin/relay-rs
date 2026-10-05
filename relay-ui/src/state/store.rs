use std::time::Duration;

use dioxus::core::spawn_forever;
use dioxus::prelude::*;
use dioxus_fullstack::ServerFnError;
use dioxus_sdk_time::sleep;
use futures_util::StreamExt;
use relay_monitor::clear_history;
use relay_settings::{restart, save_settings, settings};
use relay_targets::TargetList;
use relay_targets::{save_targets, targets};
use relay_tunnel::{restart_tunnel, tunnel, tunnel_enabled, watch_tunnel};
use relay_vpn::{VpnInfo, add_vpn, remove_vpn, vpns};

use super::{Page, SettingsDraft, Status, TargetDraft};

#[derive(Clone, Copy)]
pub(crate) struct Store {
    pub(crate) page: Signal<Page>,
    pub(crate) targets: Signal<Vec<TargetDraft>>,
    saved_targets: Signal<Vec<TargetDraft>>,
    pub(crate) settings: Signal<Option<SettingsDraft>>,
    saved_settings: Signal<Option<SettingsDraft>>,
    pub(crate) tunnel: Signal<String>,
    pub(crate) tunnel_on: Signal<bool>,
    pub(crate) vpns: Signal<Vec<VpnInfo>>,
    pub(crate) vpn_name: Signal<String>,
    pub(crate) vpn_config: Signal<String>,
    pub(crate) status: Signal<Status>,
}

impl Store {
    const RETRY: Duration = Duration::from_secs(2);

    pub(crate) fn provide() -> Self {
        let store = Self {
            page: use_signal(|| Page::Targets),
            targets: use_signal(Vec::new),
            saved_targets: use_signal(Vec::new),
            settings: use_signal(|| None),
            saved_settings: use_signal(|| None),
            tunnel: use_signal(String::new),
            tunnel_on: use_signal(|| true),
            vpns: use_signal(Vec::new),
            vpn_name: use_signal(String::new),
            vpn_config: use_signal(String::new),
            status: use_signal(|| Status::Loading),
        };
        use_hook(move || {
            store.refresh_tunnel();
            store.load();
        });
        use_context_provider(|| store)
    }

    pub(crate) fn current() -> Self {
        use_context()
    }

    pub(crate) fn show(mut self, page: Page) {
        self.page.set(page);
    }

    pub(crate) fn targets_dirty(self) -> bool {
        *self.targets.read() != *self.saved_targets.read()
    }

    pub(crate) fn settings_dirty(self) -> bool {
        *self.settings.read() != *self.saved_settings.read()
    }

    pub(crate) fn discard_targets(mut self) {
        self.targets.set(self.saved_targets.cloned());
        self.status.set(Status::Ready);
    }

    pub(crate) fn discard_settings(mut self) {
        self.settings.set(self.saved_settings.cloned());
        self.status.set(Status::Ready);
    }

    pub(crate) fn refresh_tunnel(mut self) {
        spawn(async move {
            self.tunnel_on.set(tunnel_enabled().await.unwrap_or(true));
            self.tunnel.set(tunnel().await.unwrap_or_default());
        });
    }

    pub(crate) fn restart_tunnel(mut self) {
        spawn_forever(async move {
            if let Err(error) = restart_tunnel().await {
                self.fail(error);
                return;
            }
            self.tunnel.set(String::new());
        });
    }

    pub(crate) async fn watch_tunnel(mut self) {
        loop {
            if let Ok(stream) = watch_tunnel().await {
                let mut stream = stream.into_inner();
                while let Some(Ok(url)) = stream.next().await {
                    self.tunnel.set(url);
                }
            }
            sleep(Self::RETRY).await;
            self.tunnel.set(tunnel().await.unwrap_or_default());
        }
    }

    pub(crate) fn restart_relay(mut self) {
        spawn_forever(async move {
            let _ = restart().await;
            self.status.set(Status::Restarting);
            self.tunnel.set(String::new());
            loop {
                sleep(Self::RETRY).await;
                if let Ok(loaded) = targets().await {
                    self.apply_targets(loaded);
                    self.refresh_tunnel();
                    break;
                }
            }
        });
    }

    fn load(self) {
        spawn(async move {
            match targets().await {
                Ok(loaded) => self.apply_targets(loaded),
                Err(error) => self.fail(error),
            }
        });
    }

    fn apply_targets(mut self, loaded: TargetList) {
        let drafts: Vec<TargetDraft> = loaded
            .targets
            .into_iter()
            .map(TargetDraft::from_target)
            .collect();
        self.saved_targets.set(drafts.clone());
        self.targets.set(drafts);
        self.status.set(Status::Ready);
    }

    fn fail(mut self, error: ServerFnError) {
        self.status.set(Status::Failed(Self::message(error)));
    }

    pub(crate) fn clear_monitor(self) {
        spawn(async move {
            if let Err(error) = clear_history().await {
                self.fail(error);
            }
        });
    }

    pub(crate) fn load_settings(mut self) {
        spawn(async move {
            match settings().await {
                Ok(loaded) => {
                    let draft = Some(SettingsDraft::from_settings(loaded));
                    self.saved_settings.set(draft.clone());
                    self.settings.set(draft);
                }
                Err(error) => self.fail(error),
            }
        });
    }

    fn edit_settings(mut self, edit: impl FnOnce(&mut SettingsDraft)) {
        if let Some(draft) = self.settings.write().as_mut() {
            edit(draft);
        }
        self.status.set(Status::Edited);
    }

    pub(crate) fn set_listen(self, listen: String) {
        self.edit_settings(|draft| draft.listen = listen);
    }

    pub(crate) fn set_tunnel_mode(self, tunnel: String) {
        self.edit_settings(|draft| draft.tunnel = tunnel);
    }

    pub(crate) fn set_tunnel_token(self, token: String) {
        self.edit_settings(|draft| draft.token = token);
    }

    pub(crate) fn save_settings(mut self) {
        let parsed = match self.settings.read().as_ref().map(SettingsDraft::parse) {
            Some(Ok(parsed)) => parsed,
            Some(Err(message)) => {
                self.status.set(Status::Failed(message));
                return;
            }
            None => return,
        };
        self.status.set(Status::Saving);
        spawn(async move {
            match save_settings(parsed).await {
                Ok(()) => {
                    self.saved_settings.set(self.settings.cloned());
                    self.status.set(Status::Saved);
                }
                Err(error) => self.fail(error),
            }
        });
    }

    pub(crate) fn save_targets(mut self) {
        let mut parsed = Vec::new();
        for draft in self.targets.read().iter() {
            match draft.parse() {
                Ok(target) => parsed.push(target),
                Err(message) => {
                    self.status.set(Status::Failed(message));
                    return;
                }
            }
        }
        self.status.set(Status::Saving);
        spawn(async move {
            match save_targets(TargetList { targets: parsed }).await {
                Ok(()) => {
                    self.saved_targets.set(self.targets.cloned());
                    self.status.set(Status::Saved);
                }
                Err(error) => self.fail(error),
            }
        });
    }

    fn message(error: ServerFnError) -> String {
        match error {
            ServerFnError::ServerError { message, .. } => message,
            other => other.to_string(),
        }
    }

    fn change(mut self, change: impl FnOnce(&mut Vec<TargetDraft>)) {
        self.targets.with_mut(change);
        self.status.set(Status::Edited);
    }

    fn edit(self, index: usize, edit: impl FnOnce(&mut TargetDraft)) {
        self.change(|targets| {
            if let Some(target) = targets.get_mut(index) {
                edit(target);
            }
        });
    }

    pub(crate) fn add_target(self) {
        self.change(|targets| targets.push(TargetDraft::default()));
    }

    pub(crate) fn remove_target(self, index: usize) {
        self.change(|targets| {
            if index < targets.len() {
                targets.remove(index);
            }
        });
    }

    pub(crate) fn set_prefix(self, index: usize, prefix: String) {
        self.edit(index, |target| target.prefix = prefix);
    }

    pub(crate) fn set_upstream(self, index: usize, upstream: String) {
        self.edit(index, |target| target.upstream = upstream);
    }

    pub(crate) fn add_rule(self, index: usize) {
        self.edit(index, |target| target.rules.push(String::new()));
    }

    pub(crate) fn set_rule(self, index: usize, position: usize, line: String) {
        self.edit(index, |target| {
            if let Some(rule) = target.rules.get_mut(position) {
                *rule = line;
            }
        });
    }

    pub(crate) fn remove_rule(self, index: usize, position: usize) {
        self.edit(index, |target| {
            if position < target.rules.len() {
                target.rules.remove(position);
            }
        });
    }

    pub(crate) fn set_target_vpn(self, index: usize, vpn: String) {
        self.edit(index, |target| target.vpn = vpn);
    }

    pub(crate) fn load_vpns(mut self) {
        spawn(async move {
            match vpns().await {
                Ok(loaded) => self.vpns.set(loaded),
                Err(error) => self.fail(error),
            }
        });
    }

    pub(crate) fn set_vpn_name(mut self, name: String) {
        self.vpn_name.set(name);
    }

    pub(crate) fn set_vpn_config(mut self, config: String) {
        self.vpn_config.set(config);
    }

    pub(crate) fn vpn_form_filled(self) -> bool {
        !self.vpn_name.read().trim().is_empty() && !self.vpn_config.read().trim().is_empty()
    }

    pub(crate) fn add_vpn(mut self) {
        if !self.vpn_form_filled() {
            self.status.set(Status::Failed(
                "vpn: name and config are required".to_owned(),
            ));
            return;
        }
        let (name, config) = (
            self.vpn_name.read().trim().to_owned(),
            self.vpn_config.cloned(),
        );
        self.status.set(Status::Saving);
        spawn(async move {
            match add_vpn(name, config).await {
                Ok(()) => {
                    self.vpn_name.set(String::new());
                    self.vpn_config.set(String::new());
                    self.status.set(Status::Saved);
                    self.load_vpns();
                }
                Err(error) => self.fail(error),
            }
        });
    }

    pub(crate) fn remove_vpn(self, name: String) {
        spawn(async move {
            match remove_vpn(name).await {
                Ok(()) => self.load_vpns(),
                Err(error) => self.fail(error),
            }
        });
    }
}
