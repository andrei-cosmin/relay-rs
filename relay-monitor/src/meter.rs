use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::body::{BodyDataStream, Bytes};
use futures_util::Stream;
use http_body::Body as _;
use relay_core::Body;

use crate::{Direction, EntryLog};

pub struct ByteMeter {
    inner: BodyDataStream,
    log: Arc<EntryLog>,
    id: u64,
    direction: Direction,
    finished: bool,
}

impl ByteMeter {
    pub fn wrap(body: Body, log: Arc<EntryLog>, id: u64, direction: Direction) -> Body {
        if body.is_end_stream() {
            log.update(id, |entry| entry.finish(direction));
            return body;
        }
        Body::from_stream(Self { inner: body.into_data_stream(), log, id, direction, finished: false })
    }

    fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let direction = self.direction;
        self.log.update(self.id, |entry| entry.finish(direction));
    }
}

impl Stream for ByteMeter {
    type Item = Result<Bytes, axum::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let polled = Pin::new(&mut self.inner).poll_next(cx);
        match &polled {
            Poll::Ready(Some(Ok(chunk))) => {
                let (direction, length) = (self.direction, chunk.len() as u64);
                self.log.update(self.id, |entry| entry.count(direction, length));
            }
            Poll::Ready(None) => self.finish(),
            _ => {}
        }
        polled
    }
}

impl Drop for ByteMeter {
    fn drop(&mut self) {
        self.finish();
    }
}
