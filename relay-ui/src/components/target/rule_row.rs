use dioxus::prelude::*;

use super::rule::{Rule, Verb};
use crate::components::{Chip, Field, IconButton};
use crate::state::Store;

#[component]
pub(super) fn RuleRow(target: usize, position: usize, line: String) -> Element {
    let store = Store::current();
    let rule = Rule::parse(&line);
    let verb = rule.verb;
    let write = move |line: String| store.set_rule(target, position, line);

    rsx! {
        div { class: "rule",
            Chip { tone: verb.tone(), label: verb.label() }
            match (verb.takes_args(), rule.secret()) {
                (false, _) => rsx! { span { class: "rule-fill" } },
                (true, Some((name, value))) => {
                    let (name, value) = (name.to_owned(), value.to_owned());
                    let (kept_name, kept_value) = (name.clone(), value.clone());
                    rsx! {
                        Field {
                            value: name,
                            placeholder: "header",
                            mono: true,
                            variant: "name",
                            onchange: move |name: String| write(Rule::line(Verb::Set, &format!("{name} {kept_value}"))),
                        }
                        Field {
                            value,
                            placeholder: "value",
                            kind: "password",
                            mono: true,
                            variant: "grow",
                            onchange: move |value: String| write(Rule::line(Verb::Set, &format!("{kept_name} {value}"))),
                        }
                    }
                }
                (true, None) => rsx! {
                    Field {
                        value: rule.args.to_owned(),
                        placeholder: verb.hint(),
                        kind: if rule.hidden() { "password" } else { "text" },
                        mono: true,
                        variant: "grow",
                        onchange: move |args: String| write(Rule::line(verb, &args)),
                    }
                },
            }
            IconButton { label: "✕", danger: true, onclick: move |_| store.remove_rule(target, position) }
        }
    }
}
