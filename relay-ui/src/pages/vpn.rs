use std::time::Duration;

use dioxus::prelude::*;
use dioxus_sdk_time::use_interval;

use crate::components::{Bar, Button, VpnForm, VpnList};
use crate::state::{Status, Store};

#[component]
pub(crate) fn VpnPage() -> Element {
    let store = Store::current();
    use_hook(move || store.load_vpns());
    use_interval(Duration::from_secs(3), move |()| store.load_vpns());
    let filled = store.vpn_form_filled();
    let saving = (store.status)() == Status::Saving;

    rsx! {
        main { class: "content",
            VpnList {}
            VpnForm {}
        }
        Bar {
            end: rsx! {
                Button { primary: true, disabled: !filled || saving, onclick: move |_| store.add_vpn(), "Add vpn" }
            },
        }
    }
}
