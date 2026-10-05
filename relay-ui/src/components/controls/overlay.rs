use dioxus::prelude::*;

use super::Window;

#[component]
pub(crate) fn Overlay(title: String, children: Element) -> Element {
    rsx! {
        div { class: "overlay",
            Window { title: "relay", tone: "orange",
                div { class: "modal",
                    span { class: "spinner" }
                    span { class: "modal-title", "{title}" }
                    {children}
                }
            }
        }
    }
}
