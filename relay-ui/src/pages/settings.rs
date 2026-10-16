use dioxus::prelude::*;

use crate::components::{Bar, Button, SettingsForm};
use crate::state::{Status, Store};

#[component]
pub(crate) fn SettingsPage() -> Element {
    let store = Store::current();
    use_hook(move || store.load_settings());
    let dirty = store.settings_dirty();
    let saving = (store.status)() == Status::Saving;

    rsx! {
        main { class: "content",
            SettingsForm {}
        }
        Bar {
            end: rsx! {
                Button { ghost: true, disabled: !dirty, onclick: move |_| store.discard_settings(), "Discard" }
                Button { primary: true, disabled: !dirty || saving, onclick: move |_| store.save_settings(), "Save" }
            },
        }
    }
}
