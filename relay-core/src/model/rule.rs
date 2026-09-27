use http::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub enum Rule {
    Set(HeaderName, HeaderValue),
    Remove(Vec<String>),
    Cors,
    Block,
    Key(String),
}

impl TryFrom<String> for Rule {
    type Error = String;

    fn try_from(line: String) -> Result<Self, String> {
        let (verb, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        match verb {
            "set" => {
                let (name, value) = rest.split_once(' ').ok_or(format!("{line}: set needs a name and a value"))?;
                let name = HeaderName::try_from(name).map_err(|_| format!("{line}: bad header name"))?;
                let value = HeaderValue::try_from(value.trim()).map_err(|_| format!("{line}: bad header value"))?;
                Ok(Self::Set(name, value))
            }
            "remove" if !rest.trim().is_empty() => Ok(Self::Remove(rest.split_whitespace().map(str::to_ascii_lowercase).collect())),
            "cors" => Ok(Self::Cors),
            "block" => Ok(Self::Block),
            "key" if !rest.trim().is_empty() => Ok(Self::Key(rest.trim().to_owned())),
            "key" => Err(format!("{line}: key needs a secret")),
            _ => Err(format!("{line}: unknown rule")),
        }
    }
}

impl From<Rule> for String {
    fn from(rule: Rule) -> Self {
        match rule {
            Rule::Set(name, value) => format!("set {name} {}", value.to_str().unwrap_or_default()),
            Rule::Remove(patterns) => format!("remove {}", patterns.join(" ")),
            Rule::Cors => "cors".to_owned(),
            Rule::Block => "block".to_owned(),
            Rule::Key(secret) => format!("key {secret}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_takes_a_name_and_the_rest_as_value() {
        let Ok(Rule::Set(name, value)) = Rule::try_from("set authorization Bearer a b".to_owned()) else {
            panic!("set did not parse");
        };
        assert_eq!(name, "authorization");
        assert_eq!(value, "Bearer a b");
        assert_eq!(Rule::try_from("set authorization".to_owned()).err().unwrap(), "set authorization: set needs a name and a value");
    }

    #[test]
    fn remove_lowercases_every_name() {
        let Ok(Rule::Remove(names)) = Rule::try_from("remove Origin CF-*".to_owned()) else {
            panic!("remove did not parse");
        };
        assert_eq!(names, ["origin", "cf-*"]);
        assert!(Rule::try_from("remove".to_owned()).err().unwrap().ends_with("unknown rule"));
    }

    #[test]
    fn key_keeps_its_secret_trimmed() {
        let Ok(Rule::Key(secret)) = Rule::try_from("key  hunter2 ".to_owned()) else {
            panic!("key did not parse");
        };
        assert_eq!(secret, "hunter2");
        assert_eq!(Rule::try_from("key".to_owned()).err().unwrap(), "key: key needs a secret");
    }

    #[test]
    fn bare_verbs_and_unknown_ones() {
        assert!(matches!(Rule::try_from("cors".to_owned()), Ok(Rule::Cors)));
        assert!(matches!(Rule::try_from("block".to_owned()), Ok(Rule::Block)));
        assert_eq!(Rule::try_from("frob x".to_owned()).err().unwrap(), "frob x: unknown rule");
    }

    #[test]
    fn text_round_trips() {
        for line in ["set user-agent node-fetch", "remove origin cf-*", "cors", "block", "key hunter2"] {
            assert_eq!(String::from(Rule::try_from(line.to_owned()).unwrap()), line);
        }
    }
}
