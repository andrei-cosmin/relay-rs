use dioxus::prelude::*;

#[component]
pub(crate) fn Chip(tone: String, label: String) -> Element {
    rsx! {
        span { class: "chip chip-{tone}", "{label}" }
    }
}
