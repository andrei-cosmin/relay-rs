use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::http::header::CONTENT_LENGTH;
use futures_util::{Stream, stream};
use haze::Pack;
use relay_core::{Body, Ctx, DataDir, Error, Incoming, Interceptor, Outgoing, Reply};
use relay_storage::{Page, RdbStore};
use tokio::time::MissedTickBehavior;

use crate::{ByteMeter, Direction, Entry, Event};

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
        self.persist(&entry);
        self.live().insert(entry.id, entry);
        self.changed();
    }

    pub fn update(&self, id: u64, change: impl FnOnce(&mut Entry)) {
        let mut live = self.live();
        let Some(entry) = live.get_mut(&id) else {
            return;
        };
        let stage = entry.stage;
        change(entry);
        let finished = entry.finished();
        let changed = if finished {
            live.remove(&id)
        } else if entry.stage != stage {
            Some(entry.clone())
        } else {
            None
        };
        drop(live);
        if let Some(changed) = changed {
            self.persist(&changed);
            if finished {
                tracing::info!("{}", changed.line());
                self.trim(changed.id);
            }
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

    pub fn changes(&self) -> impl Stream<Item = u64> + Send + 'static {
        let mut ticker = tokio::time::interval(Self::TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        stream::unfold(
            (self.clone(), ticker, None),
            |(monitor, mut ticker, mut seen)| async move {
                loop {
                    ticker.tick().await;
                    let version = monitor.version.load(Ordering::Relaxed);
                    if seen == Some(version) {
                        continue;
                    }
                    seen = Some(version);
                    return Some((version, (monitor, ticker, seen)));
                }
            },
        )
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
        let expected = incoming
            .headers
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()?.parse().ok());
        self.update(ctx.id, |entry| {
            entry.apply(Event::Answered(status, expected))
        });
        Ok(())
    }

    fn tap_request(&self, ctx: &Ctx, body: Body) -> Body {
        ByteMeter::wrap(body, self.clone(), ctx.id, Direction::Sent)
    }

    fn tap_response(&self, ctx: &Ctx, body: Body) -> Body {
        ByteMeter::wrap(body, self.clone(), ctx.id, Direction::Received)
    }

    fn on_cancel(&self, ctx: &Ctx) {
        self.update(ctx.id, |entry| entry.apply(Event::Cancelled));
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use axum::body::Bytes;
    use futures_util::StreamExt;

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
            expected: None,
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

    fn answered_reply(monitor: &Monitor, chunks: Vec<Result<Bytes, io::Error>>) -> Body {
        reply_of(monitor, 200, None, chunks)
    }

    fn reply_of(
        monitor: &Monitor,
        status: u16,
        expected: Option<u64>,
        chunks: Vec<Result<Bytes, io::Error>>,
    ) -> Body {
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.apply(Event::Answered(status, expected)));
        let body = Body::from_stream(stream::iter(chunks));
        ByteMeter::wrap(body, monitor.clone(), 1, Direction::Received)
    }

    fn two_chunks() -> Vec<Result<Bytes, io::Error>> {
        vec![Ok(Bytes::from_static(b"ab")), Ok(Bytes::from_static(b"cd"))]
    }

    #[tokio::test]
    async fn a_reply_read_to_the_end_is_done() {
        let monitor = monitor("reply-done");
        let mut data = answered_reply(&monitor, two_chunks()).into_data_stream();
        while data.next().await.is_some() {}
        let stored = monitor.entry(1).unwrap();
        assert!(stored.stage == Stage::Done && stored.received == 4);
    }

    #[tokio::test]
    async fn a_reply_dropped_halfway_is_cancelled() {
        let monitor = monitor("reply-dropped");
        let mut data = answered_reply(&monitor, two_chunks()).into_data_stream();
        data.next().await;
        drop(data);
        let stored = monitor.entry(1).unwrap();
        assert!(stored.stage == Stage::Cancelled && stored.received == 2);
        assert!(monitor.live().is_empty());
    }

    #[tokio::test]
    async fn a_sized_error_dropped_after_its_last_byte_is_failed() {
        let monitor = monitor("reply-sized");
        let chunks = vec![Ok(Bytes::from_static(
            b"{\"error\":\"Internal Server Error\"}",
        ))];
        let mut data = reply_of(&monitor, 500, Some(33), chunks).into_data_stream();
        data.next().await;
        drop(data);
        let stored = monitor.entry(1).unwrap();
        assert!(stored.stage == Stage::Failed && stored.received == 33);
        assert!(monitor.live().is_empty());
    }

    #[tokio::test]
    async fn a_reply_that_breaks_upstream_is_failed() {
        let monitor = monitor("reply-broken");
        let chunks = vec![
            Ok(Bytes::from_static(b"ab")),
            Err(io::Error::other("upstream reset")),
        ];
        let mut data = answered_reply(&monitor, chunks).into_data_stream();
        while data.next().await.is_some() {}
        drop(data);
        assert!(monitor.entry(1).unwrap().stage == Stage::Failed);
    }

    #[test]
    fn a_cancelled_request_leaves_memory() {
        let monitor = monitor("cancelled");
        monitor.record(entry(1));
        monitor.update(1, |entry| entry.apply(Event::Cancelled));
        assert!(monitor.live().is_empty());
        assert!(monitor.entry(1).unwrap().stage == Stage::Cancelled);
    }

    #[tokio::test(start_paused = true)]
    async fn changes_sends_the_current_version_then_only_new_ones() {
        let monitor = monitor("changes");
        let mut changes = Box::pin(monitor.changes());
        let first = changes.next().await.unwrap();
        monitor.record(entry(1));
        let second = changes.next().await.unwrap();
        assert!(second > first);
        let quiet = tokio::time::timeout(Duration::from_secs(5), changes.next()).await;
        assert!(quiet.is_err());
    }
}
