use dioxus::prelude::*;

use super::{RuleRow, TargetHeader};
use crate::components::{Button, Window};
use crate::state::{Store, TargetDraft};

#[component]
pub(crate) fn TargetCard(index: usize, target: TargetDraft) -> Element {
    let store = Store::current();

    rsx! {
        Window { title: "target {index + 1}", tone: "yellow",
            TargetHeader { index, prefix: target.prefix, upstream: target.upstream, vpn: target.vpn }
            div { class: "rules",
                for (position, line) in target.rules.into_iter().enumerate() {
                    RuleRow { key: "{position}", target: index, position, line }
                }
            }
            div { class: "target-foot",
                Button { ghost: true, onclick: move |_| store.add_rule(index), "+ rule" }
            }
        }
    }
}
