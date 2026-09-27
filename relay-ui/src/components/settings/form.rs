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

    rsx! {
        Window { title: "settings", tone: "amber",
            div { class: "setting",
                span { class: "setting-label", "listen" }
                Field {
                    value: draft.listen.clone(),
                    placeholder: "127.0.0.1:8585",
                    mono: true,
                    variant: "grow",
                    onchange: move |listen| store.set_listen(listen),
                }
            }
            div { class: "setting",
                span { class: "setting-label", "admin" }
                Field {
                    value: draft.admin.clone(),
                    placeholder: "0.0.0.0:8080",
                    mono: true,
                    variant: "grow",
                    onchange: move |admin| store.set_admin(admin),
                }
            }
            div { class: "setting",
                span { class: "setting-label", "tunnel" }
                Select {
                    value: draft.tunnel.clone(),
                    none: SettingsDraft::OFF,
                    options: vec![SettingsDraft::QUICK.to_owned(), SettingsDraft::DOMAIN.to_owned()],
                    onchange: move |mode| store.set_tunnel_mode(mode),
                }
            }
            if draft.wants_token() {
                div { class: "setting",
                    span { class: "setting-label", "token" }
                    Field {
                        value: draft.token.clone(),
                        placeholder: "tunnel token from the Cloudflare dashboard",
                        kind: "password",
                        mono: true,
                        variant: "grow",
                        onchange: move |token| store.set_tunnel_token(token),
                    }
                }
                p { class: "hint", "In Cloudflare, point the tunnel's public hostname at http://{draft.listen}" }
            }
            p { class: "hint", "Ports and tunnel apply after a restart" }
            div { class: "settings-actions",
                span { class: "spacer" }
                Button { onclick: move |_| store.restart_tunnel(), "Restart tunnel" }
                Button { ghost: true, onclick: move |_| store.restart_relay(), "Restart relay" }
            }
        }
    }
}
