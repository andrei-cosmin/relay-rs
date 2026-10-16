use dioxus::prelude::*;

use crate::components::{Button, Field, Select, Window};
use crate::state::{SettingsDraft, Store};

#[component]
pub(crate) fn SettingsForm() -> Element {
    let store = Store::current();
    let Some(draft) = (store.settings)() else {
        return rsx! {
            p { class: "notice", "Loading settings…" }
        };
    };
    let wants_token = draft.wants_token();
    let SettingsDraft {
        listen,
        tunnel,
        token,
        idle_timeout,
    } = draft;

    rsx! {
        Window { title: "settings", tone: "amber",
            div { class: "setting",
                span { class: "setting-label", "listen" }
                Field {
                    value: listen.clone(),
                    placeholder: "127.0.0.1:8585",
                    mono: true,
                    variant: "grow",
                    onchange: move |listen| store.set_listen(listen),
                }
            }
            div { class: "setting",
                span { class: "setting-label", "tunnel" }
                Select {
                    value: tunnel,
                    none: SettingsDraft::OFF,
                    options: vec![SettingsDraft::QUICK.to_owned(), SettingsDraft::DOMAIN.to_owned()],
                    onchange: move |mode| store.set_tunnel_mode(mode),
                }
            }
            if wants_token {
                div { class: "setting",
                    span { class: "setting-label", "token" }
                    Field {
                        value: token,
                        placeholder: "tunnel token from the Cloudflare dashboard",
                        kind: "password",
                        mono: true,
                        variant: "grow",
                        onchange: move |token| store.set_tunnel_token(token),
                    }
                }
                p { class: "hint", "In Cloudflare, point the tunnel's public hostname at http://{listen}" }
            }
            div { class: "setting",
                span { class: "setting-label", "idle timeout" }
                Field {
                    value: idle_timeout,
                    placeholder: "0",
                    mono: true,
                    variant: "grow",
                    onchange: move |idle_timeout| store.set_idle_timeout(idle_timeout),
                }
            }
            p { class: "hint", "Idle timeout: seconds an upstream may stay silent before relay gives up; every packet restarts the count, 0 is off. Applies to new requests as soon as it is saved." }
            p { class: "hint", "Listen address applies after a restart" }
            div { class: "settings-actions",
                span { class: "spacer" }
                Button { onclick: move |_| store.restart_tunnel(), "Restart tunnel" }
                Button { ghost: true, onclick: move |_| store.restart_relay(), "Restart relay" }
            }
        }
    }
}
