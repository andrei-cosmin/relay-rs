use dioxus::prelude::*;

use crate::components::{IconButton, Pill, Tab};
use crate::state::{Page, Store};

#[component]
pub(crate) fn Header() -> Element {
    let store = Store::current();
    use_future(move || store.watch_tunnel());
    let tunnel = (store.tunnel)();
    let live = !tunnel.is_empty();
    let on = (store.tunnel_on)();
    let current = (store.page)();

    rsx! {
        header { class: "top",
            span { class: "brand",
                span { class: "brand-mark" }
                "Relay-RS"
            }
            nav { class: "tabs",
                for page in Page::ALL {
                    Tab { key: "{page.label()}", label: page.label(), active: page == current, onclick: move |_| store.show(page) }
                }
            }
            span { class: "spacer" }
            if on {
                Pill { tone: if live { "live" } else { "starting" }, dot: true,
                    span { class: "tunnel-url", if live { "{tunnel}" } else { "Starting…" } }
                    IconButton { label: "↺", onclick: move |_| store.restart_tunnel() }
                }
            } else {
                Pill { tone: "neutral", dot: true,
                    span { class: "tunnel-url", "No tunnel" }
                }
            }
        }
    }
}
