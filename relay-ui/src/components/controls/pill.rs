use dioxus::prelude::*;

#[component]
pub(crate) fn Pill(tone: String, #[props(default)] dot: bool, children: Element) -> Element {
    rsx! {
        div { class: "pill pill-{tone}",
            if dot {
                span { class: "dot" }
            }
            {children}
        }
    }
}
