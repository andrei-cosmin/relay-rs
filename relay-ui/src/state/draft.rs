use relay_core::Target;
use serde::Serialize;

#[derive(Clone, PartialEq, Default, Serialize)]
pub(crate) struct TargetDraft {
    pub(crate) prefix: String,
    pub(crate) upstream: String,
    pub(crate) rules: Vec<String>,
    pub(crate) vpn: String,
}

impl TargetDraft {
    pub(super) fn from_target(target: Target) -> Self {
        Self {
            prefix: target.prefix,
            upstream: target.upstream,
            rules: target.rules.into_iter().map(String::from).collect(),
            vpn: target.vpn.unwrap_or_default(),
        }
    }

    pub(super) fn parse(&self) -> Result<Target, String> {
        let mut value = serde_json::to_value(self).expect("draft serialize");
        if self.vpn.is_empty() {
            value["vpn"] = serde_json::Value::Null;
        }
        serde_json::from_value(value).map_err(|error| error.to_string())
    }
}
