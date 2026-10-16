use dioxus::prelude::*;

use super::VpnRow;
use crate::components::Window;
use crate::state::Store;

#[component]
pub(crate) fn VpnList() -> Element {
    let store = Store::current();
    let vpns = (store.vpns)();

    rsx! {
        Window { title: "vpns", tone: "orange",
            if vpns.is_empty() {
                p { class: "notice", "No vpns yet. Paste a WireGuard config below." }
            }
            for vpn in vpns {
                VpnRow { key: "{vpn.name}", name: vpn.name, up: vpn.up }
            }
        }
    }
}
