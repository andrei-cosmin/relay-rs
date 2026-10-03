use dioxus::prelude::*;

use crate::components::{Field, Window};
use crate::state::Store;

#[component]
pub(crate) fn VpnForm() -> Element {
    let store = Store::current();
    let config = (store.vpn_config)();

    rsx! {
        Window { title: "add vpn", tone: "amber",
            div { class: "setting",
                span { class: "setting-label", "name" }
                Field {
                    value: (store.vpn_name)(),
                    placeholder: "proton-nl",
                    mono: true,
                    variant: "grow",
                    onchange: move |name| store.set_vpn_name(name),
                }
            }
            div { class: "field-area",
                if config.is_empty() {
                    span { class: "field-hint", "[Interface]\nPrivateKey = …\nAddress = 10.2.0.2/32\n\n[Peer]\nPublicKey = …\nEndpoint = 203.0.113.7:51820" }
                }
                textarea {
                    class: "field-area-input",
                    value: "{config}",
                    oninput: move |event| store.set_vpn_config(event.value()),
                }
            }
            p { class: "hint", "paste a WireGuard config; a SOCKS5 port is added for you; the private key never leaves the relay" }
        }
    }
}
