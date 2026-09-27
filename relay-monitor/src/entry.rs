use serde::{Deserialize, Serialize};

use crate::Human;

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum Stage {
    Received,
    Forwarded,
    Answered,
    Done,
    Failed,
}

#[cfg(feature = "server")]
#[derive(Clone, Copy)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct Entry {
    pub id: u64,
    pub method: String,
    pub path: String,
    pub prefix: String,
    pub stage: Stage,
    pub status: Option<u16>,
    pub started: u64,
    pub elapsed: u64,
    pub sent: u64,
    pub received: u64,
}

impl Entry {
    pub fn finished(&self) -> bool {
        matches!(self.stage, Stage::Done | Stage::Failed)
    }

    pub fn status_label(&self) -> String {
        self.status.map_or_else(|| "—".to_owned(), |status| status.to_string())
    }

    pub fn elapsed_label(&self) -> String {
        if self.status.is_none() {
            return "—".to_owned();
        }
        Human::duration(self.elapsed)
    }

    pub fn line(&self) -> String {
        format!("{} {} {} {} {} {} up {} down", self.id, self.method, self.path, self.status_label(), self.elapsed_label(), Human::size(self.sent), Human::size(self.received))
    }
}

#[cfg(feature = "server")]
impl relay_storage::Record for Entry {
    const TABLE: &'static str = "requests";
    const VERSION: u32 = 1;

    fn key(&self) -> u64 {
        self.id
    }
}

#[cfg(feature = "server")]
impl Entry {
    pub fn received(ctx: &relay_core::Ctx) -> Self {
        Self {
            id: ctx.id,
            method: ctx.method.to_string(),
            path: ctx.path.clone(),
            prefix: ctx.target.prefix.clone(),
            stage: Stage::Received,
            status: None,
            started: Self::now(),
            elapsed: 0,
            sent: 0,
            received: 0,
        }
    }

    pub fn count(&mut self, direction: Direction, length: u64) {
        match direction {
            Direction::Sent => self.sent += length,
            Direction::Received => self.received += length,
        }
    }

    pub fn finish(&mut self, direction: Direction) {
        match direction {
            Direction::Sent => {
                if self.stage == Stage::Received {
                    self.stage = Stage::Forwarded;
                }
            }
            Direction::Received => {
                self.elapsed = Self::now().saturating_sub(self.started);
                self.stage = if self.status.is_some_and(|status| status >= 500) { Stage::Failed } else { Stage::Done };
            }
        }
    }

    pub fn answered(&mut self, status: u16) {
        self.status = Some(status);
        self.elapsed = Self::now().saturating_sub(self.started);
        self.stage = Stage::Answered;
    }

    fn now() -> u64 {
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_millis() as u64,
            Err(_) => 0,
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    fn entry() -> Entry {
        Entry { id: 1, method: "POST".into(), path: "/sm/x".into(), prefix: "/sm".into(), stage: Stage::Received, status: None, started: Entry::now(), elapsed: 0, sent: 0, received: 0 }
    }

    #[test]
    fn bytes_add_up_per_direction() {
        let mut entry = entry();
        entry.count(Direction::Sent, 10);
        entry.count(Direction::Sent, 5);
        entry.count(Direction::Received, 7);
        assert_eq!((entry.sent, entry.received), (15, 7));
    }

    #[test]
    fn stages_follow_the_request() {
        let mut entry = entry();
        entry.finish(Direction::Sent);
        assert!(entry.stage == Stage::Forwarded);
        entry.answered(200);
        assert!(entry.stage == Stage::Answered && entry.status == Some(200));
        entry.finish(Direction::Received);
        assert!(entry.stage == Stage::Done);
    }

    #[test]
    fn a_server_error_fails_once_its_body_is_through() {
        let mut entry = entry();
        entry.answered(503);
        entry.count(Direction::Received, 9);
        assert!(entry.stage == Stage::Answered);
        entry.finish(Direction::Received);
        assert!(entry.stage == Stage::Failed && entry.received == 9);
    }
}
