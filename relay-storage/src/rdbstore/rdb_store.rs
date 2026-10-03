use std::{fmt::Display, fs, marker::PhantomData, path::Path, sync::Arc};

use redb::{
    Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, TableError,
};
use relay_core::Error;

use super::Record;
use crate::Page;

pub struct RdbStore<T> {
    database: Arc<Database>,
    table: String,
    record: PhantomData<fn() -> T>,
}

impl<T: Record> RdbStore<T> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .map_err(|error| Error::internal(format!("{}: {error}", parent.display())))?;
        }
        let database = Database::create(path)
            .map_err(|error| Error::internal(format!("{}: {error}", path.display())))?;
        Ok(Self {
            database: Arc::new(database),
            table: T::table(),
            record: PhantomData,
        })
    }

    pub fn put(&self, record: &T) -> Result<(), Error> {
        let bytes = postcard::to_allocvec(record).map_err(Self::fail)?;
        let txn = self.database.begin_write().map_err(Self::fail)?;
        {
            let mut table = txn.open_table(self.definition()).map_err(Self::fail)?;
            table
                .insert(record.key(), bytes.as_slice())
                .map_err(Self::fail)?;
        }
        txn.commit().map_err(Self::fail)
    }

    pub fn get(&self, key: u64) -> Result<Option<T>, Error> {
        let txn = self.database.begin_read().map_err(Self::fail)?;
        let Some(table) = Self::existing(txn.open_table(self.definition()))? else {
            return Ok(None);
        };
        match table.get(key).map_err(Self::fail)? {
            Some(bytes) => Ok(postcard::from_bytes(bytes.value()).ok()),
            None => Ok(None),
        }
    }

    pub fn page(&self, number: usize, size: usize) -> Result<Page<T>, Error> {
        let txn = self.database.begin_read().map_err(Self::fail)?;
        let Some(table) = Self::existing(txn.open_table(self.definition()))? else {
            return Ok(Page {
                number,
                size,
                ..Page::default()
            });
        };
        let mut items = Vec::with_capacity(size);
        for row in table
            .iter()
            .map_err(Self::fail)?
            .rev()
            .skip(number * size)
            .take(size)
        {
            let (_, bytes) = row.map_err(Self::fail)?;
            if let Ok(record) = postcard::from_bytes(bytes.value()) {
                items.push(record);
            }
        }
        Ok(Page {
            number,
            size,
            total: table.len().map_err(Self::fail)?,
            items,
        })
    }

    pub fn count(&self) -> Result<u64, Error> {
        let txn = self.database.begin_read().map_err(Self::fail)?;
        let Some(table) = Self::existing(txn.open_table(self.definition()))? else {
            return Ok(0);
        };
        table.len().map_err(Self::fail)
    }

    pub fn prune(&self, before: u64) -> Result<u64, Error> {
        let txn = self.database.begin_write().map_err(Self::fail)?;
        let removed = {
            let mut table = txn.open_table(self.definition()).map_err(Self::fail)?;
            table
                .extract_from_if(..before, |_, _| true)
                .map_err(Self::fail)?
                .count() as u64
        };
        txn.commit().map_err(Self::fail)?;
        Ok(removed)
    }

    pub fn newest_key(&self) -> Result<Option<u64>, Error> {
        let txn = self.database.begin_read().map_err(Self::fail)?;
        let Some(table) = Self::existing(txn.open_table(self.definition()))? else {
            return Ok(None);
        };
        Ok(table
            .last()
            .map_err(Self::fail)?
            .map(|(key, _)| key.value()))
    }

    pub fn clear(&self) -> Result<(), Error> {
        let txn = self.database.begin_write().map_err(Self::fail)?;
        txn.delete_table(self.definition()).map_err(Self::fail)?;
        txn.commit().map_err(Self::fail)
    }

    fn definition(&self) -> TableDefinition<'_, u64, &'static [u8]> {
        TableDefinition::new(&self.table)
    }

    fn existing<Table>(opened: Result<Table, TableError>) -> Result<Option<Table>, Error> {
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

impl<T> Clone for RdbStore<T> {
    fn clone(&self) -> Self {
        Self {
            database: Arc::clone(&self.database),
            table: self.table.clone(),
            record: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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

    fn fresh(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("relay-rdbstore-{name}-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    fn filled(name: &str, count: u64) -> RdbStore<Note> {
        let notes = RdbStore::<Note>::open(fresh(name)).unwrap();
        for id in 1..=count {
            notes.put(&note(id)).unwrap();
        }
        notes
    }

    fn note(id: u64) -> Note {
        Note {
            id,
            text: format!("note {id}"),
        }
    }

    fn ids(page: &Page<Note>) -> Vec<u64> {
        page.items.iter().map(|note| note.id).collect()
    }

    #[test]
    fn unknown_table_reads_as_empty() {
        let notes = RdbStore::<Note>::open(fresh("empty")).unwrap();
        assert_eq!(notes.count().unwrap(), 0);
        assert!(notes.page(0, 5).unwrap().items.is_empty());
        assert_eq!(notes.get(1).unwrap(), None);
    }

    #[test]
    fn put_then_get_and_overwrite_by_key() {
        let notes = filled("put", 1);
        assert_eq!(notes.get(1).unwrap(), Some(note(1)));
        notes
            .put(&Note {
                id: 1,
                text: "changed".into(),
            })
            .unwrap();
        assert_eq!(notes.get(1).unwrap().unwrap().text, "changed");
        assert_eq!(notes.count().unwrap(), 1);
    }

    #[test]
    fn pages_run_newest_first() {
        let notes = filled("pages", 5);
        let first = notes.page(0, 2).unwrap();
        assert_eq!(
            (ids(&first), first.total, first.pages()),
            (vec![5, 4], 5, 3)
        );
        assert_eq!(ids(&notes.page(1, 2).unwrap()), [3, 2]);
        assert_eq!(ids(&notes.page(2, 2).unwrap()), [1]);
        assert!(notes.page(3, 2).unwrap().items.is_empty());
    }

    #[test]
    fn newer_records_only_push_older_ones_down() {
        let notes = filled("shift", 5);
        let before = ids(&notes.page(1, 2).unwrap());
        notes.put(&note(6)).unwrap();
        let after = ids(&notes.page(1, 2).unwrap());
        assert_eq!((before, after), (vec![3, 2], vec![4, 3]));
    }

    #[test]
    fn prune_drops_keys_below() {
        let notes = filled("prune", 5);
        assert_eq!(notes.prune(3).unwrap(), 2);
        assert_eq!(ids(&notes.page(0, 10).unwrap()), [5, 4, 3]);
    }

    #[test]
    fn a_new_version_starts_a_separate_table() {
        let path = fresh("versions");
        {
            let notes = RdbStore::<Note>::open(&path).unwrap();
            notes.put(&note(1)).unwrap();
        }
        let flags = RdbStore::<Flag>::open(&path).unwrap();
        flags.put(&Flag { id: 7, on: true }).unwrap();
        assert_eq!(flags.count().unwrap(), 1);
        assert!(flags.get(1).unwrap().is_none());
    }

    #[test]
    fn rows_that_do_not_decode_are_skipped() {
        let path = fresh("shape");
        {
            RdbStore::<Note>::open(&path)
                .unwrap()
                .put(&note(1))
                .unwrap();
        }
        let shapes = RdbStore::<Shape>::open(&path).unwrap();
        assert!(shapes.page(0, 5).unwrap().items.is_empty());
        assert!(shapes.get(1).unwrap().is_none());
    }

    #[test]
    fn newest_key_follows_the_highest_id() {
        let notes = RdbStore::<Note>::open(fresh("newest")).unwrap();
        assert_eq!(notes.newest_key().unwrap(), None);
        notes.put(&note(7)).unwrap();
        notes.put(&note(3)).unwrap();
        assert_eq!(notes.newest_key().unwrap(), Some(7));
    }

    #[test]
    fn clear_drops_the_whole_table() {
        let notes = filled("clear", 1);
        notes.clear().unwrap();
        assert_eq!(notes.count().unwrap(), 0);
        assert_eq!(notes.newest_key().unwrap(), None);
    }

    #[test]
    fn data_survives_reopen() {
        let path = fresh("reopen");
        RdbStore::<Note>::open(&path)
            .unwrap()
            .put(&note(1))
            .unwrap();
        assert_eq!(RdbStore::<Note>::open(&path).unwrap().count().unwrap(), 1);
    }
}
