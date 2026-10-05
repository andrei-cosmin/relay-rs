use dioxus::prelude::*;

#[component]
pub(crate) fn Button(
    #[props(default)] primary: bool,
    #[props(default)] ghost: bool,
    #[props(default)] disabled: bool,
    onclick: EventHandler<()>,
    children: Element,
) -> Element {
    let tone = match (primary, ghost) {
        (true, _) => "btn primary",
        (false, true) => "btn ghost",
        (false, false) => "btn",
    };
    let class = if disabled {
        format!("{tone} disabled")
    } else {
        tone.to_owned()
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
