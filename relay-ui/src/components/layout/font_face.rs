use dioxus::prelude::*;

#[component]
pub(crate) fn FontFace(family: String, weight: u16, src: Asset) -> Element {
    rsx! {
        document::Style { "@font-face {{ font-family: \"{family}\"; font-weight: {weight}; src: url(\"{src}\") format(\"woff2\"); }}" }
    }
}
