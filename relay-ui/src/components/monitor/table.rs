use dioxus::prelude::*;
use relay_monitor::changes;

use super::MonitorView;

#[component]
pub(crate) fn MonitorTable() -> Element {
    let mut number = use_signal(|| 0_usize);
    let mut version = use_signal(|| 0_u64);
    haze::use_streaming(changes, move |fresh: u64| version.set(fresh));
    rsx! {
        MonitorView { number, version, on_page: move |page| number.set(page) }
    }
}
