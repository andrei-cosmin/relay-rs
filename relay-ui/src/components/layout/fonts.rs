use dioxus::prelude::*;

use super::FontFace;

#[component]
pub(crate) fn Fonts() -> Element {
    rsx! {
        FontFace { family: "Chakra Petch", weight: 500, src: asset!("/assets/fonts/ChakraPetch-500.woff2") }
        FontFace { family: "Chakra Petch", weight: 600, src: asset!("/assets/fonts/ChakraPetch-600.woff2") }
        FontFace { family: "Chakra Petch", weight: 700, src: asset!("/assets/fonts/ChakraPetch-700.woff2") }
        FontFace { family: "Space Mono", weight: 400, src: asset!("/assets/fonts/SpaceMono-400.woff2") }
        FontFace { family: "Space Mono", weight: 700, src: asset!("/assets/fonts/SpaceMono-700.woff2") }
    }
}
