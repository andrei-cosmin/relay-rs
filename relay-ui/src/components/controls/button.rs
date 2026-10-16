use dioxus::prelude::*;

#[component]
pub(crate) fn Button(
    #[props(default)] primary: bool,
    #[props(default)] ghost: bool,
    #[props(default)] disabled: bool,
    onclick: EventHandler<()>,
    children: Element,
) -> Element {
    let class = match (primary, ghost, disabled) {
        (true, _, false) => "btn primary",
        (true, _, true) => "btn primary disabled",
        (false, true, false) => "btn ghost",
        (false, true, true) => "btn ghost disabled",
        (false, false, false) => "btn",
        (false, false, true) => "btn disabled",
    };

    rsx! {
        button {
            r#type: "button",
            class,
            disabled,
            onclick: move |_| onclick.call(()),
            {children}
        }
    }
}
