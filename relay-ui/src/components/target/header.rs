use dioxus::prelude::*;

use crate::components::{Field, IconButton, Select};
use crate::state::Store;

#[component]
pub(super) fn TargetHeader(index: usize, prefix: String, upstream: String, vpn: String) -> Element {
    let store = Store::current();
    let names: Vec<String> = store
        .vpns
        .read()
        .iter()
        .map(|info| info.name.clone())
        .collect();
    let routed = !vpn.is_empty();

    rsx! {
        div { class: "target-head",
            Field {
                value: prefix,
                placeholder: "/prefix",
                mono: true,
                variant: "prefix",
                onchange: move |prefix| store.set_prefix(index, prefix),
            }
            span { class: "arrow", "→" }
            Field {
                value: upstream,
                placeholder: "https://upstream.example",
                mono: true,
                variant: "grow",
                onchange: move |upstream| store.set_upstream(index, upstream),
            }
            div { class: if routed { "vpn-pick" } else { "vpn-pick direct" },
                Select { value: vpn, none: "direct", options: names, onchange: move |name| store.set_target_vpn(index, name) }
                IconButton { label: "⟲", onclick: move |_| store.set_target_vpn(index, String::new()) }
            }
            IconButton { label: "✕", danger: true, onclick: move |_| store.remove_target(index) }
        }
    }
}
