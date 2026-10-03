use dioxus::prelude::*;

#[component]
pub(crate) fn Select(
    value: String,
    none: String,
    options: Vec<String>,
    onchange: EventHandler<String>,
) -> Element {
    let mut open = use_signal(|| false);
    let missing = !value.is_empty() && !options.contains(&value);
    let shown = if value.is_empty() {
        none.clone()
    } else {
        value.clone()
    };
    let chosen = value.clone();

    rsx! {
        div { class: if missing { "select select-missing" } else { "select" },
            button { r#type: "button", class: "select-trigger", onclick: move |_| open.toggle(),
                span { class: "select-value", "{shown}" }
                span { class: "select-caret", "▾" }
            }
            if open() {
                div { class: "select-scrim", onclick: move |_| open.set(false) }
                div { class: "select-popup",
                    button {
                        r#type: "button",
                        class: if chosen.is_empty() { "select-option selected" } else { "select-option" },
                        onclick: move |_| {
                            open.set(false);
                            onchange.call(String::new());
                        },
                        "{none}"
                    }
                    for name in options {
                        button {
                            key: "{name}",
                            r#type: "button",
                            class: if name == chosen { "select-option selected" } else { "select-option" },
                            onclick: {
                                let name = name.clone();
                                move |_| {
                                    open.set(false);
                                    onchange.call(name.clone());
                                }
                            },
                            "{name}"
                        }
                    }
                    if missing {
                        span { class: "select-option missing", "{chosen} (missing)" }
                    }
                }
            }
        }
    }
}
