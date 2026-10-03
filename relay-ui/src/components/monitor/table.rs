use dioxus::prelude::*;

use super::MonitorView;

#[component]
pub(crate) fn MonitorTable() -> Element {
    let mut number = use_signal(|| 0_usize);
    rsx! {
        MonitorView { key: "{number}", number: number(), on_page: move |page| number.set(page) }
    }
}
