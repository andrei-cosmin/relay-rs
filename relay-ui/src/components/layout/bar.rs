use dioxus::prelude::*;

use crate::components::Pill;
use crate::state::Store;

#[component]
pub(crate) fn Bar(start: Option<Element>, end: Option<Element>) -> Element {
    let store = Store::current();
    let status = (store.status)();

    rsx! {
        footer { class: "bar",
            div { class: "bar-start", {start} }
            Pill { tone: status.tone(), dot: true, "{status.label()}" }
            div { class: "bar-end", {end} }
        }
    }
}
