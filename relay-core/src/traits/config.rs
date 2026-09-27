use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

use ron::ser::PrettyConfig;
use serde::{Serialize, de::DeserializeOwned};

use crate::Error;

pub trait Config: Serialize + DeserializeOwned + Default {
    const FILE: &'static str;

    fn path() -> String {
        format!("data/{}", Self::FILE)
    }

    fn load() -> Result<Self, Error> {
        Self::load_from(&Self::path())
    }

    fn save(&self) -> Result<(), Error> {
        self.save_to(&Self::path())
    }

    fn load_from(path: &str) -> Result<Self, Error> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
            Err(error) => return Err(Error::internal(format!("{path}: {error}"))),
        };
        let loaded = if text.trim().is_empty() {
            Self::default()
        } else {
            ron::from_str(&text).map_err(|error| Error::bad_request(format!("{path}: {error}")))?
        };
        if loaded.render()? != text.trim() {
            loaded.save_to(path)?;
        }
        Ok(loaded)
    }

    fn save_to(&self, path: &str) -> Result<(), Error> {
        let fail = |error: std::io::Error| Error::internal(format!("{path}: {error}"));
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(fail)?;
        }
        let temporary = format!("{path}.tmp");
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporary).map_err(fail)?;
        file.write_all(self.render()?.as_bytes()).map_err(fail)?;
        fs::rename(&temporary, path).map_err(fail)
    }

    fn render(&self) -> Result<String, Error> {
        ron::ser::to_string_pretty(self, PrettyConfig::default()).map_err(|error| Error::internal(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    struct Sample {
        port: u16,
        name: String,
    }

    impl Default for Sample {
        fn default() -> Self {
            Self { port: 8585, name: "relay".into() }
        }
    }

    impl Config for Sample {
        const FILE: &'static str = "sample.ron";
    }

    fn fresh(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("relay-config-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("sample.ron").to_string_lossy().into_owned()
    }

    #[test]
    fn a_missing_file_is_created_from_the_defaults() {
        let path = fresh("missing");
        assert_eq!(Sample::load_from(&path).unwrap(), Sample::default());
        assert_eq!(fs::read_to_string(&path).unwrap(), Sample::default().render().unwrap());
    }

    #[test]
    fn missing_fields_are_filled_in_and_written_back() {
        let path = fresh("partial");
        fs::create_dir_all(Path::new(&path).parent().unwrap()).unwrap();
        fs::write(&path, "(port: 9000)").unwrap();
        assert_eq!(Sample::load_from(&path).unwrap(), Sample { port: 9000, name: "relay".into() });
        assert!(fs::read_to_string(&path).unwrap().contains("name: \"relay\""));
    }

    #[test]
    fn saved_files_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let path = fresh("private");
        Sample::default().save_to(&path).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn a_broken_file_is_an_error_and_left_alone() {
        let path = fresh("broken");
        fs::create_dir_all(Path::new(&path).parent().unwrap()).unwrap();
        fs::write(&path, "(port: ").unwrap();
        assert!(Sample::load_from(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "(port: ");
    }
}
