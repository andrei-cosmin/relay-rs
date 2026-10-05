use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::Rule;

#[derive(Clone, Deserialize, Serialize)]
pub struct Target {
    #[serde(deserialize_with = "Target::normalise")]
    pub prefix: String,
    pub upstream: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub vpn: Option<String>,
}

impl Target {
    pub fn normalise<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
        let prefix = String::deserialize(deserializer)?;
        if !prefix.starts_with('/') {
            return Err(D::Error::custom(format!(
                "{prefix}: prefix must start with /"
            )));
        }
        let without_trailing_slash = prefix.trim_end_matches('/');
        if without_trailing_slash.is_empty() {
            return Ok("/".to_owned());
        }
        Ok(without_trailing_slash.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use serde::de::{IntoDeserializer, value::Error};

    use super::*;

    fn normalise(prefix: &str) -> Result<String, Error> {
        Target::normalise(prefix.into_deserializer())
    }

    #[test]
    fn trailing_slashes_are_dropped() {
        assert_eq!(normalise("/sm/").unwrap(), "/sm");
        assert_eq!(normalise("/sm").unwrap(), "/sm");
    }

    #[test]
    fn root_stays_root() {
        assert_eq!(normalise("/").unwrap(), "/");
        assert_eq!(normalise("///").unwrap(), "/");
    }

    #[test]
    fn prefix_must_start_with_a_slash() {
        assert_eq!(
            normalise("sm").unwrap_err().to_string(),
            "sm: prefix must start with /"
        );
    }
}
