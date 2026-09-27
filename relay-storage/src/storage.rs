use std::{fmt::Display, fs, path::Path};

use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, TableError};
use relay_core::Error;

use crate::Record;

pub struct Storage {
    db: Database,
}

impl Storage {
    pub fn open(path: &str) -> Result<Self, Error> {
        if let Some(parent) = Path::new(path).parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|error| Error::internal(format!("{}: {error}", parent.display())))?;
        }
        let db = Database::create(path).map_err(|error| Error::internal(format!("{path}: {error}")))?;
        Ok(Self { db })
    }

    pub fn put<T: Record>(&self, record: &T) -> Result<(), Error> {
        let bytes = postcard::to_allocvec(record).map_err(Self::fail)?;
        let name = T::table();
        let txn = self.db.begin_write().map_err(Self::fail)?;
        {
            let mut table = txn.open_table(Self::definition(&name)).map_err(Self::fail)?;
            table.insert(record.key(), bytes.as_slice()).map_err(Self::fail)?;
        }
        txn.commit().map_err(Self::fail)
    }

    pub fn get<T: Record>(&self, key: u64) -> Result<Option<T>, Error> {
        let name = T::table();
        let txn = self.db.begin_read().map_err(Self::fail)?;
        let Some(table) = Self::existing(txn.open_table(Self::definition(&name)))? else {
            return Ok(None);
        };
        match table.get(key).map_err(Self::fail)? {
            Some(bytes) => Ok(postcard::from_bytes(bytes.value()).ok()),
            None => Ok(None),
        }
    }

    pub fn page<T: Record>(&self, page: usize, size: usize) -> Result<Vec<T>, Error> {
        let name = T::table();
        let txn = self.db.begin_read().map_err(Self::fail)?;
        let Some(table) = Self::existing(txn.open_table(Self::definition(&name)))? else {
            return Ok(Vec::new());
        };
        let mut records = Vec::with_capacity(size);
        for row in table.iter().map_err(Self::fail)?.rev().skip(page * size).take(size) {
            let (_, bytes) = row.map_err(Self::fail)?;
            if let Ok(record) = postcard::from_bytes(bytes.value()) {
                records.push(record);
            }
        }
        Ok(records)
    }

    pub fn count<T: Record>(&self) -> Result<u64, Error> {
        let name = T::table();
        let txn = self.db.begin_read().map_err(Self::fail)?;
        let Some(table) = Self::existing(txn.open_table(Self::definition(&name)))? else {
            return Ok(0);
        };
        table.len().map_err(Self::fail)
    }

    pub fn prune<T: Record>(&self, before: u64) -> Result<u64, Error> {
        let name = T::table();
        let txn = self.db.begin_write().map_err(Self::fail)?;
        let removed = {
            let mut table = txn.open_table(Self::definition(&name)).map_err(Self::fail)?;
            table.extract_from_if(..before, |_, _| true).map_err(Self::fail)?.count() as u64
        };
        txn.commit().map_err(Self::fail)?;
        Ok(removed)
    }

    pub fn newest_key<T: Record>(&self) -> Result<Option<u64>, Error> {
        let name = T::table();
        let txn = self.db.begin_read().map_err(Self::fail)?;
        let Some(table) = Self::existing(txn.open_table(Self::definition(&name)))? else {
            return Ok(None);
        };
        Ok(table.last().map_err(Self::fail)?.map(|(key, _)| key.value()))
    }

    pub fn clear<T: Record>(&self) -> Result<(), Error> {
        let name = T::table();
        let txn = self.db.begin_write().map_err(Self::fail)?;
        txn.delete_table(Self::definition(&name)).map_err(Self::fail)?;
        txn.commit().map_err(Self::fail)
    }

    fn definition(name: &str) -> TableDefinition<'_, u64, &'static [u8]> {
        TableDefinition::new(name)
    }

    fn existing<T>(opened: Result<T, TableError>) -> Result<Option<T>, Error> {
        match opened {
            Ok(table) => Ok(Some(table)),
            Err(TableError::TableDoesNotExist(_)) => Ok(None),
            Err(error) => Err(Self::fail(error)),
        }
    }

    fn fail(error: impl Display) -> Error {
        Error::internal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Note {
        id: u64,
        text: String,
    }

    impl Record for Note {
        const TABLE: &'static str = "notes";
        const VERSION: u32 = 1;

        fn key(&self) -> u64 {
            self.id
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Flag {
        id: u64,
        on: bool,
    }

    impl Record for Flag {
        const TABLE: &'static str = "notes";
        const VERSION: u32 = 2;

        fn key(&self) -> u64 {
            self.id
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Shape {
        id: u64,
        on: bool,
    }

    impl Record for Shape {
        const TABLE: &'static str = "notes";
        const VERSION: u32 = 1;

        fn key(&self) -> u64 {
            self.id
        }
    }

    fn fresh(name: &str) -> (Storage, String) {
        let path = std::env::temp_dir().join(format!("relay-storage-{name}-{}.redb", std::process::id()));
        let path = path.to_string_lossy().into_owned();
        let _ = std::fs::remove_file(&path);
        (Storage::open(&path).unwrap(), path)
    }

    fn note(id: u64) -> Note {
        Note { id, text: format!("note {id}") }
    }

    fn ids(notes: Vec<Note>) -> Vec<u64> {
        notes.into_iter().map(|note| note.id).collect()
    }

    #[test]
    fn unknown_table_reads_as_empty() {
        let (storage, _) = fresh("empty");
        assert_eq!(storage.count::<Note>().unwrap(), 0);
        assert!(storage.page::<Note>(0, 5).unwrap().is_empty());
        assert_eq!(storage.get::<Note>(1).unwrap(), None);
    }

    #[test]
    fn put_then_get_and_overwrite_by_key() {
        let (storage, _) = fresh("put");
        storage.put(&note(1)).unwrap();
        assert_eq!(storage.get::<Note>(1).unwrap(), Some(note(1)));
        storage.put(&Note { id: 1, text: "changed".into() }).unwrap();
        assert_eq!(storage.get::<Note>(1).unwrap().unwrap().text, "changed");
        assert_eq!(storage.count::<Note>().unwrap(), 1);
    }

    #[test]
    fn pages_run_newest_first() {
        let (storage, _) = fresh("pages");
        for id in 1..=5 {
            storage.put(&note(id)).unwrap();
        }
        assert_eq!(ids(storage.page::<Note>(0, 2).unwrap()), [5, 4]);
        assert_eq!(ids(storage.page::<Note>(1, 2).unwrap()), [3, 2]);
        assert_eq!(ids(storage.page::<Note>(2, 2).unwrap()), [1]);
        assert!(storage.page::<Note>(3, 2).unwrap().is_empty());
    }

    #[test]
    fn prune_drops_keys_below() {
        let (storage, _) = fresh("prune");
        for id in 1..=5 {
            storage.put(&note(id)).unwrap();
        }
        assert_eq!(storage.prune::<Note>(3).unwrap(), 2);
        assert_eq!(ids(storage.page::<Note>(0, 10).unwrap()), [5, 4, 3]);
    }

    #[test]
    fn versions_live_in_separate_tables() {
        let (storage, _) = fresh("versions");
        storage.put(&note(1)).unwrap();
        storage.put(&Flag { id: 7, on: true }).unwrap();
        assert_eq!(storage.count::<Note>().unwrap(), 1);
        assert_eq!(storage.count::<Flag>().unwrap(), 1);
        assert!(storage.get::<Flag>(1).unwrap().is_none());
    }

    #[test]
    fn rows_that_do_not_decode_are_skipped() {
        let (storage, _) = fresh("shape");
        storage.put(&note(1)).unwrap();
        assert!(storage.page::<Shape>(0, 5).unwrap().is_empty());
        assert!(storage.get::<Shape>(1).unwrap().is_none());
    }

    #[test]
    fn newest_key_follows_the_highest_id() {
        let (storage, _) = fresh("newest");
        assert_eq!(storage.newest_key::<Note>().unwrap(), None);
        storage.put(&note(7)).unwrap();
        storage.put(&note(3)).unwrap();
        assert_eq!(storage.newest_key::<Note>().unwrap(), Some(7));
    }

    #[test]
    fn clear_drops_the_whole_table() {
        let (storage, _) = fresh("clear");
        storage.put(&note(1)).unwrap();
        storage.clear::<Note>().unwrap();
        assert_eq!(storage.count::<Note>().unwrap(), 0);
        assert_eq!(storage.newest_key::<Note>().unwrap(), None);
    }

    #[test]
    fn data_survives_reopen() {
        let (storage, path) = fresh("reopen");
        storage.put(&note(1)).unwrap();
        drop(storage);
        assert_eq!(Storage::open(&path).unwrap().count::<Note>().unwrap(), 1);
    }
}
