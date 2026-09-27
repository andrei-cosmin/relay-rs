use dioxus::prelude::*;

use super::MonitorRow;
use crate::components::{Button, Window};
use crate::state::Store;

#[component]
pub(crate) fn MonitorTable() -> Element {
    let store = Store::current();
    let history = (store.history)();
    use_future(move || store.watch_monitor());
    let page = history.page;
    let pages = history.pages();

    rsx! {
        Window { title: "recent requests", tone: "orange",
            div { class: "monitor-bar",
                Button { ghost: true, disabled: page == 0, onclick: move |_| store.show_history(page.saturating_sub(1)), "‹" }
                span { class: "mono pager", "{page + 1} / {pages}" }
                Button { ghost: true, disabled: page + 1 >= pages, onclick: move |_| store.show_history(page + 1), "›" }
                span { class: "spacer" }
                Button { ghost: true, onclick: move |_| store.clear_monitor(), "clear" }
            }
            div { class: "table",
                div { class: "table-head",
                    span { "id" }
                    span { "method" }
                    span { "path" }
                    span { "prefix" }
                    span { "stage" }
                    span { "status" }
                    span { "elapsed" }
                    span { "sent" }
                    span { "received" }
                }
                div { class: "table-body",
                    if history.entries.is_empty() {
                        p { class: "notice", "No requests yet." }
                    }
                    for entry in history.entries {
                        MonitorRow { key: "{entry.id}", entry }
                    }
                }
            }
        }
    }
}
