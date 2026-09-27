use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};

use futures_util::Stream;
use relay_core::{Error, Feed};
use relay_storage::Storage;

use crate::{Entry, History};

pub struct EntryLog {
    live: Mutex<BTreeMap<u64, Entry>>,
    feed: Feed<Entry>,
    storage: Arc<Storage>,
}

impl EntryLog {
    const PAGE: usize = 20;
    const KEEP: u64 = 10_000;
    const PRUNE_EVERY: u64 = 256;

    pub fn new(storage: Arc<Storage>) -> Self {
        Self { live: Mutex::new(BTreeMap::new()), feed: Feed::new(256), storage }
    }

    pub fn push(&self, entry: Entry) {
        self.live().insert(entry.id, entry.clone());
        self.persist(&entry);
        self.feed.push(entry);
    }

    pub fn update(&self, id: u64, change: impl FnOnce(&mut Entry)) {
        let mut live = self.live();
        let Some(entry) = live.get_mut(&id) else {
            return;
        };
        let stage = entry.stage;
        change(entry);
        let changed = entry.clone();
        if changed.finished() {
            live.remove(&id);
        }
        drop(live);
        if changed.stage != stage {
            self.persist(&changed);
        }
        if changed.finished() {
            println!("{}", changed.line());
            self.trim(changed.id);
        }
        self.feed.push(changed);
    }

    pub fn history(&self, page: usize) -> Result<History, Error> {
        Ok(History { page, size: Self::PAGE, total: self.storage.count::<Entry>()?, entries: self.storage.page::<Entry>(page, Self::PAGE)? })
    }

    pub fn clear(&self) -> Result<(), Error> {
        self.live().clear();
        self.storage.clear::<Entry>()
    }

    pub fn watch(&self) -> impl Stream<Item=Entry> + Send + 'static {
        self.feed.subscribe()
    }

    fn persist(&self, entry: &Entry) {
        if let Err(error) = self.storage.put(entry) {
            println!("history: {error}");
        }
    }

    fn trim(&self, newest: u64) {
        if newest % Self::PRUNE_EVERY != 0 {
            return;
        }
        if let Err(error) = self.storage.prune::<Entry>(newest.saturating_sub(Self::KEEP)) {
            println!("history: {error}");
        }
    }

    fn live(&self) -> MutexGuard<'_, BTreeMap<u64, Entry>> {
        self.live.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use crate::Stage;

    use super::*;

    fn log(name: &str) -> EntryLog {
        let path = std::env::temp_dir().join(format!("relay-monitor-{name}-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        EntryLog::new(Arc::new(Storage::open(&path.to_string_lossy()).unwrap()))
    }

    fn entry(id: u64) -> Entry {
        Entry { id, method: "GET".into(), path: "/x".into(), prefix: "/".into(), stage: Stage::Received, status: None, started: 0, elapsed: 0, sent: 0, received: 0 }
    }

    fn ids(history: History) -> Vec<u64> {
        history.entries.into_iter().map(|entry| entry.id).collect()
    }

    #[test]
    fn history_comes_in_pages_of_twenty_newest_first() {
        let log = log("pages");
        for id in 1..=25 {
            log.push(entry(id));
        }
        let first = log.history(0).unwrap();
        assert_eq!((first.total, first.pages(), first.entries.len()), (25, 2, 20));
        assert_eq!(first.entries[0].id, 25);
        assert_eq!(ids(log.history(1).unwrap()), [5, 4, 3, 2, 1]);
    }

    #[test]
    fn stage_changes_are_stored_and_byte_counts_ride_along() {
        let log = log("stages");
        log.push(entry(1));
        log.update(1, |entry| entry.sent = 9);
        assert_eq!(log.history(0).unwrap().entries[0].sent, 0);
        log.update(1, |entry| entry.stage = Stage::Done);
        let stored = log.history(0).unwrap().entries.remove(0);
        assert!(stored.stage == Stage::Done && stored.sent == 9);
    }

    #[test]
    fn finished_entries_stop_taking_updates() {
        let log = log("finished");
        log.push(entry(1));
        log.update(1, |entry| entry.stage = Stage::Failed);
        log.update(1, |entry| entry.stage = Stage::Done);
        assert!(log.history(0).unwrap().entries[0].stage == Stage::Failed);
    }

    #[test]
    fn clear_empties_the_history() {
        let log = log("clear");
        log.push(entry(1));
        log.clear().unwrap();
        assert_eq!(log.history(0).unwrap().total, 0);
    }
}
