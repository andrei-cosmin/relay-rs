use dioxus::prelude::*;

#[component]
pub(crate) fn Field(
    value: String,
    placeholder: String,
    #[props(default)] kind: Option<String>,
    #[props(default)] mono: bool,
    #[props(default)] variant: Option<String>,
    onchange: EventHandler<String>,
) -> Element {
    let masked = kind.as_deref() == Some("password");
    let shown = if masked {
        Mask(&value).shown()
    } else {
        value.clone()
    };
    let class = match (variant, mono) {
        (Some(variant), true) => format!("field mono field-{variant}"),
        (Some(variant), false) => format!("field field-{variant}"),
        (None, true) => "field mono".to_owned(),
        (None, false) => "field".to_owned(),
    };

    rsx! {
        div { class,
            if value.is_empty() {
                span { class: "field-hint", "{placeholder}" }
            }
            input {
                class: "field-input",
                r#type: "text",
                value: "{shown}",
                oninput: move |event| {
                    let typed = event.value();
                    onchange.call(if masked { Mask(&value).apply(&typed) } else { typed })
                },
            }
        }
    }
}

struct Mask<'a>(&'a str);

impl Mask<'_> {
    const BULLET: char = '•';

    fn shown(&self) -> String {
        self.0.chars().map(|_| Self::BULLET).collect()
    }

    fn apply(&self, typed: &str) -> String {
        let real: Vec<char> = self.0.chars().collect();
        let typed: Vec<char> = typed.chars().collect();
        let front = typed
            .iter()
            .take_while(|&&c| c == Self::BULLET)
            .count()
            .min(real.len());
        let back = typed[front..]
            .iter()
            .rev()
            .take_while(|&&c| c == Self::BULLET)
            .count()
            .min(real.len() - front);
        real[..front]
            .iter()
            .chain(&typed[front..typed.len() - back])
            .chain(&real[real.len() - back..])
            .collect()
    }
}
