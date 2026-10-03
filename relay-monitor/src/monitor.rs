use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use dioxus_fullstack::SseTx;
use haze::Pack;
use relay_core::{Body, Ctx, DataDir, Error, Incoming, Interceptor, Outgoing, Reply};
use relay_storage::{Page, RdbStore};
use tokio::time::MissedTickBehavior;

use crate::{ByteMeter, Direction, Entry};

#[derive(Clone, Pack)]
pub struct Monitor {
    store: RdbStore<Entry>,
    #[pack(func = Arc::default())]
    live: Arc<Mutex<BTreeMap<u64, Entry>>>,
    #[pack(func = Arc::default())]
    version: Arc<AtomicU64>,
}

#[haze::resource]
fn history(dir: DataDir) -> Result<RdbStore<Entry>, Error> {
    RdbStore::open(dir.0.join("relay.redb"))
}

impl Monitor {
    const PAGE: usize = 20;
    const KEEP: u64 = 10_000;
    const PRUNE_EVERY: u64 = 256;
    const TICK: Duration = Duration::from_millis(500);

    pub fn newest_id(&self) -> Result<Option<u64>, Error> {
        self.store.newest_key()
    }

    pub fn record(&self, entry: Entry) {
        self.live().insert(entry.id, entry.clone());
        self.persist(&entry);
        self.changed();
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
            tracing::info!("{}", changed.line());
            self.trim(changed.id);
        }
        self.changed();
    }

    pub fn page(&self, number: usize) -> Result<Page<Entry>, Error> {
        let mut page = self.store.page(number, Self::PAGE)?;
        let live = self.live();
        for row in &mut page.items {
            if let Some(fresh) = live.get(&row.id) {
                *row = fresh.clone();
            }
        }
        Ok(page)
    }

    pub fn entry(&self, id: u64) -> Option<Entry> {
        if let Some(fresh) = self.live().get(&id) {
            return Some(fresh.clone());
        }
        self.store.get(id).ok().flatten()
    }

    pub fn clear(&self) -> Result<(), Error> {
        self.live().clear();
        self.store.clear()?;
        self.changed();
        Ok(())
    }

    pub async fn follow(&self, number: usize, mut sender: SseTx<Page<Entry>>) {
        let mut ticker = tokio::time::interval(Self::TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut seen = None;
        loop {
            ticker.tick().await;
            let version = self.version.load(Ordering::Relaxed);
            if seen == Some(version) {
                continue;
            }
            seen = Some(version);
            let Ok(page) = self.page(number) else {
                continue;
            };
            if sender.send(page).await.is_err() {
                return;
            }
        }
    }

    fn changed(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    fn persist(&self, entry: &Entry) {
        if let Err(error) = self.store.put(entry) {
            tracing::error!("history: {}", error.message);
        }
    }

    fn trim(&self, newest: u64) {
        if !newest.is_multiple_of(Self::PRUNE_EVERY) {
            return;
        }
        match self.store.prune(newest.saturating_sub(Self::KEEP)) {
            Ok(0) => {}
            Ok(_) => self.changed(),
            Err(error) => tracing::error!("history: {}", error.message),
        }
    }

    fn live(&self) -> MutexGuard<'_, BTreeMap<u64, Entry>> {
        self.live
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[haze::register(order = 20)]
impl Interceptor for Monitor {
    fn on_request(&self, ctx: &Ctx, _outgoing: &mut Outgoing) -> Result<(), Box<Reply>> {
        self.record(Entry::new(ctx));
        Ok(())
    }

    fn on_response(&self, ctx: &Ctx, incoming: &mut Incoming) -> Result<(), Box<Reply>> {
        let status = incoming.status.as_u16();
        self.update(ctx.id, |entry| entry.set_answered(status));
        Ok(())
    }

    fn tap_request(&self, ctx: &Ctx, body: Body) -> Body {
        ByteMeter::wrap(body, self.clone(), ctx.id, Direction::Sent)
    }

    fn tap_response(&self, ctx: &Ctx, body: Body) -> Body {
        ByteMeter::wrap(body, self.clone(), ctx.id, Direction::Received)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Stage;

    fn monitor(name: &str) -> Monitor {
        let path =
            std::env::temp_dir().join(format!("relay-monitor-{name}-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Monitor {
            store: RdbStore::open(path).unwrap(),
            live: Arc::default(),
            version: Arc::default(),
        }
    }

    fn entry(id: u64) -> Entry {
        Entry {
            id,
            method: "GET".into(),
            path: "/x".into(),
            prefix: "/".into(),
            stage: Stage::Received,
            status: None,
            started: 0,
            elapsed: 0,
            sent: 0,
            received: 0,
        }
    }

    fn ids(page: &Page<Entry>) -> Vec<u64> {
        page.items.iter().map(|entry| entry.id).collect()
    }

    #[test]
    fn pages_hold_twenty_newest_first() {
        let monitor = monitor("pages");
        for id in 1..=25 {
            monitor.record(entry(id));
        }
        let first = monitor.page(0).unwrap();
        assert_eq!(
            (
                first.total,
                first.pages(),
                first.items.len(),
                first.items[0].id
            ),
            (25, 2, 20, 25)
        );
        assert_eq!(ids(&monitor.page(1).unwrap()), [5, 4, 3, 2, 1]);
    }

    #[test]
    fn pages_show_live_byte_counts_before_they_are_stored() {
        let monitor = monitor("live");
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.sent = 9);
        assert_eq!(monitor.page(0).unwrap().items[0].sent, 9);
        assert_eq!(monitor.store.get(1).unwrap().unwrap().sent, 0);
    }

    #[test]
    fn finishing_stores_the_final_counts_and_leaves_memory() {
        let monitor = monitor("finish");
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.sent = 9);
        monitor.update(1, |entry| entry.stage = Stage::Done);
        assert!(monitor.live().is_empty());
        let stored = monitor.store.get(1).unwrap().unwrap();
        assert!(stored.stage == Stage::Done && stored.sent == 9);
    }

    #[test]
    fn finished_entries_stop_taking_updates() {
        let monitor = monitor("finished");
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.stage = Stage::Failed);
        monitor.update(1, |entry| entry.stage = Stage::Done);
        assert!(monitor.entry(1).unwrap().stage == Stage::Failed);
    }

    #[test]
    fn clear_empties_everything() {
        let monitor = monitor("clear");
        monitor.record(entry(1));
        monitor.clear().unwrap();
        assert_eq!(monitor.page(0).unwrap().total, 0);
    }

    #[test]
    fn every_change_moves_the_version() {
        let monitor = monitor("version");
        let start = monitor.version.load(Ordering::Relaxed);
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.sent = 1);
        monitor.clear().unwrap();
        assert_eq!(monitor.version.load(Ordering::Relaxed), start + 3);
    }
}
