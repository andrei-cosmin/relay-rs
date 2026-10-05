use dioxus::prelude::*;
use relay_monitor::{Entry, Human, Stage};

use crate::components::Chip;

#[component]
pub(super) fn MonitorRow(entry: Entry) -> Element {
    let (tone, label) = match entry.stage {
        Stage::Received => ("neutral", "RECEIVED"),
        Stage::Forwarded => ("neutral", "FORWARDED"),
        Stage::Answered => ("info", "ANSWERED"),
        Stage::Done => ("ok", "DONE"),
        Stage::Failed => ("bad", "FAILED"),
    };

    rsx! {
        div { class: "table-row",
            span { class: "num", "{entry.id}" }
            span { class: "method", "{entry.method}" }
            span { class: "path", "{entry.path}" }
            span { class: "mono", "{entry.prefix}" }
            Chip { tone, label }
            span { class: "num", "{entry.status_label()}" }
            span { class: "num", "{entry.elapsed_label()}" }
            span { class: "num", "{Human::size(entry.sent)}" }
            span { class: "num", "{Human::size(entry.received)}" }
        }
    }
}
