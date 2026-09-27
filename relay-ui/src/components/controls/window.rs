use dioxus::prelude::*;

#[component]
pub(crate) fn Window(title: String, #[props(
    default
)] tone: Option<String>, children: Element) -> Element {
    let class = match tone {
        Some(tone) => format!("window window-{tone}"),
        None => "window".to_owned(),
    };

    rsx! {
        section { class,
            div { class: "window-title",
                span { class: "window-name", "{title}" }
                span { class: "window-dots",
                    i {}
                    i {}
                    i {}
                }
            }
            div { class: "window-body", {children} }
        }
    }
}
