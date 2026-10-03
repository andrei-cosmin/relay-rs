use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard},
};

use relay_core::Error;
use ron::ser::PrettyConfig;
use serde::{Serialize, de::DeserializeOwned};

pub struct FileStore<T> {
    path: Arc<Path>,
    value: Arc<RwLock<T>>,
}

impl<T: Serialize + DeserializeOwned + Default> FileStore<T> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        Ok(Self {
            path: Arc::from(path),
            value: Arc::new(RwLock::new(Self::load(path)?)),
        })
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.value.read().expect("file store lock")
    }

    pub fn set(&self, value: T) -> Result<(), Error> {
        let mut current = self.value.write().expect("file store lock");
        Self::write(&self.path, &value)?;
        *current = value;
        Ok(())
    }

    pub fn reload(&self) -> Result<(), Error> {
        let fresh = Self::load(&self.path)?;
        *self.value.write().expect("file store lock") = fresh;
        Ok(())
    }

    fn load(path: &Path) -> Result<T, Error> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
            Err(error) => return Err(Error::internal(format!("{}: {error}", path.display()))),
        };
        let loaded = if text.trim().is_empty() {
            T::default()
        } else {
            ron::from_str(&text)
                .map_err(|error| Error::bad_request(format!("{}: {error}", path.display())))?
        };
        if Self::render(&loaded)? != text.trim() {
            Self::write(path, &loaded)?;
        }
        Ok(loaded)
    }

    fn write(path: &Path, value: &T) -> Result<(), Error> {
        let fail = |error: std::io::Error| Error::internal(format!("{}: {error}", path.display()));
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(fail)?;
        }
        let mut temporary = OsString::from(path.as_os_str());
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(fail)?;
        file.write_all(Self::render(value)?.as_bytes())
            .map_err(fail)?;
        fs::rename(&temporary, path).map_err(fail)
    }

    fn render(value: &T) -> Result<String, Error> {
        ron::ser::to_string_pretty(value, PrettyConfig::default())
            .map_err(|error| Error::internal(error.to_string()))
    }
}

impl<T> Clone for FileStore<T> {
    fn clone(&self) -> Self {
        Self {
            path: Arc::clone(&self.path),
            value: Arc::clone(&self.value),
        }
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
            Self {
                port: 8585,
                name: "relay".into(),
            }
        }
    }

    fn fresh(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("relay-filestore-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("sample.ron")
    }

    #[test]
    fn a_missing_file_is_created_from_the_defaults() {
        let path = fresh("missing");
        let store = FileStore::<Sample>::open(&path).unwrap();
        assert_eq!(*store.read(), Sample::default());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            FileStore::<Sample>::render(&Sample::default()).unwrap()
        );
    }

    #[test]
    fn missing_fields_are_filled_in_and_written_back() {
        let path = fresh("partial");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "(port: 9000)").unwrap();
        let store = FileStore::<Sample>::open(&path).unwrap();
        assert_eq!(
            *store.read(),
            Sample {
                port: 9000,
                name: "relay".into()
            }
        );
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("name: \"relay\"")
        );
    }

    #[test]
    fn set_saves_the_file_and_the_value() {
        let path = fresh("set");
        let store = FileStore::<Sample>::open(&path).unwrap();
        store
            .set(Sample {
                port: 1,
                name: "one".into(),
            })
            .unwrap();
        assert_eq!(store.read().port, 1);
        assert_eq!(FileStore::<Sample>::open(&path).unwrap().read().name, "one");
    }

    #[test]
    fn reload_picks_up_edits_to_the_file() {
        let path = fresh("reload");
        let store = FileStore::<Sample>::open(&path).unwrap();
        fs::write(&path, "(port: 7000, name: \"edited\")").unwrap();
        store.reload().unwrap();
        assert_eq!(
            *store.read(),
            Sample {
                port: 7000,
                name: "edited".into()
            }
        );
    }

    #[test]
    fn clones_share_the_value_and_the_file() {
        let path = fresh("clone");
        let store = FileStore::<Sample>::open(&path).unwrap();
        let other = store.clone();
        other
            .set(Sample {
                port: 2,
                name: "two".into(),
            })
            .unwrap();
        assert_eq!(store.read().port, 2);
    }

    #[test]
    fn saved_files_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let path = fresh("private");
        FileStore::<Sample>::open(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn a_broken_file_is_an_error_and_left_alone() {
        let path = fresh("broken");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "(port: ").unwrap();
        assert!(FileStore::<Sample>::open(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "(port: ");
    }
}
