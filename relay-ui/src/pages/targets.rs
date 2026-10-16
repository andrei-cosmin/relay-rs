use dioxus::prelude::*;

use crate::components::{Bar, Button, TargetCard};
use crate::state::{Status, Store};

#[component]
pub(crate) fn TargetsPage() -> Element {
    let store = Store::current();
    use_hook(move || store.load_vpns());
    let targets = (store.targets)();
    let empty = targets.is_empty() && (store.status)() != Status::Loading;
    let dirty = store.targets_dirty();
    let saving = (store.status)() == Status::Saving;

    rsx! {
        main { class: "content",
            for (index, target) in targets.into_iter().enumerate() {
                TargetCard { key: "{index}", index, target }
            }
            if empty {
                p { class: "notice", "No targets yet. Add one with + target." }
            }
        }
        Bar {
            start: rsx! { Button { onclick: move |_| store.add_target(), "+ target" } },
            end: rsx! {
                Button { ghost: true, disabled: !dirty, onclick: move |_| store.discard_targets(), "Discard" }
                Button { primary: true, disabled: !dirty || saving, onclick: move |_| store.save_targets(), "Save" }
            },
        }
    }
}
