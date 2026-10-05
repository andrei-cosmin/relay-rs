use dioxus::prelude::*;
use relay_monitor::{Entry, Page, watch};

use super::MonitorRow;
use crate::components::{Button, Window};
use crate::state::Store;

#[component]
pub(super) fn MonitorView(number: usize, on_page: EventHandler<usize>) -> Element {
    let store = Store::current();
    let mut shown = use_signal(Page::<Entry>::default);
    haze::use_streaming(
        move || watch(number),
        move |fresh: Page<Entry>| shown.set(fresh),
    );
    let page = shown();
    let pages = page.pages();

    rsx! {
        Window { title: "recent requests", tone: "orange",
            div { class: "monitor-bar",
                Button { ghost: true, disabled: number == 0, onclick: move |_| on_page.call(number.saturating_sub(1)), "‹" }
                span { class: "mono pager", "{number + 1} / {pages}" }
                Button { ghost: true, disabled: number + 1 >= pages, onclick: move |_| on_page.call(number + 1), "›" }
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
                    if page.items.is_empty() {
                        p { class: "notice", "No requests yet." }
                    }
                    for entry in page.items {
                        MonitorRow { key: "{entry.id}", entry }
                    }
                }
            }
        }
    }
}
