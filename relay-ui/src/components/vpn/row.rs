use dioxus::prelude::*;

use crate::components::{IconButton, Pill};
use crate::state::Store;

#[component]
pub(super) fn VpnRow(name: String, up: bool) -> Element {
    let store = Store::current();
    let removed = name.clone();

    rsx! {
        div { class: "vpn-row",
            span { class: "mono", "{name}" }
            Pill { tone: if up { "live" } else { "starting" }, dot: true, if up { "up" } else { "starting…" } }
            IconButton { label: "✕", danger: true, onclick: move |_| store.remove_vpn(removed.clone()) }
        }
    }
}
