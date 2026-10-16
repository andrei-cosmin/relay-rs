use dioxus::prelude::*;

use crate::components::{Bar, MonitorTable};

#[component]
pub(crate) fn MonitorPage() -> Element {
    rsx! {
        main { class: "content fill",
            MonitorTable {}
        }
        Bar {}
    }
}
