use dioxus::prelude::*;

#[component]
pub(crate) fn Tab(label: String, active: bool, onclick: EventHandler<()>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if active { "tab active" } else { "tab" },
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}
