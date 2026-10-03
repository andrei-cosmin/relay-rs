use dioxus::prelude::*;

use crate::components::{Fonts, Header, Overlay};
use crate::pages::{MonitorPage, SettingsPage, TargetsPage, VpnPage};
use crate::state::{Page, Status, Store};

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    let store = Store::provide();

    rsx! {
        Fonts {}
        document::Stylesheet { href: MAIN_CSS }
        div { class: "app",
            Header {}
            match (store.page)() {
                Page::Targets => rsx! { TargetsPage {} },
                Page::Monitor => rsx! { MonitorPage {} },
                Page::Vpn => rsx! { VpnPage {} },
                Page::Settings => rsx! { SettingsPage {} },
            }
            if (store.status)() == Status::Restarting {
                Overlay { title: "Restarting…",
                    p { class: "muted", "the relay is coming back on its own" }
                }
            }
        }
    }
}
