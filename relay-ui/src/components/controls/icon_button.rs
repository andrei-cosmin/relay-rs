use dioxus::prelude::*;

#[component]
pub(crate) fn IconButton(
    label: String,
    #[props(default)] danger: bool,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if danger { "icon-btn danger" } else { "icon-btn" },
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}
