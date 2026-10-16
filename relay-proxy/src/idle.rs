use std::{
    error,
    io::{self, ErrorKind},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use axum::{body::Bytes, http::StatusCode};
use futures_util::Stream;
use relay_core::{Body, Error};
use tokio::time::{Instant, Sleep};

#[derive(Clone, Default)]
pub struct IdleTimeout(Arc<AtomicU64>);

impl IdleTimeout {
    pub fn new(seconds: u64) -> Self {
        Self(Arc::new(AtomicU64::new(seconds)))
    }

    pub fn set(&self, seconds: u64) {
        self.0.store(seconds, Ordering::Relaxed);
    }

    pub(crate) fn limit(&self) -> IdleLimit {
        match self.0.load(Ordering::Relaxed) {
            0 => IdleLimit(None),
            seconds => IdleLimit(Some(Duration::from_secs(seconds))),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct IdleLimit(Option<Duration>);

impl IdleLimit {
    pub(crate) async fn wait<F: IntoFuture>(self, future: F) -> Result<F::Output, Error> {
        let Some(limit) = self.0 else {
            return Ok(future.await);
        };
        tokio::time::timeout(limit, future)
            .await
            .map_err(|_| Error {
                status: StatusCode::GATEWAY_TIMEOUT,
                message: format!("upstream sent nothing for {}s", limit.as_secs()),
            })
    }

    pub(crate) fn watch<S, E>(self, stream: S) -> Body
    where
        S: Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: Into<Box<dyn error::Error + Send + Sync>> + 'static,
    {
        match self.0 {
            Some(limit) => Body::from_stream(IdleStream::new(stream, limit)),
            None => Body::from_stream(stream),
        }
    }
}

struct IdleStream<S> {
    inner: Pin<Box<S>>,
    limit: Duration,
    deadline: Pin<Box<Sleep>>,
    expired: bool,
}

impl<S> IdleStream<S> {
    fn new(inner: S, limit: Duration) -> Self {
        Self {
            inner: Box::pin(inner),
            limit,
            deadline: Box::pin(tokio::time::sleep(limit)),
            expired: false,
        }
    }
}

impl<S, E> Stream for IdleStream<S>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<Box<dyn error::Error + Send + Sync>>,
{
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        if this.expired {
            return Poll::Ready(None);
        }
        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(item) => {
                this.deadline.as_mut().reset(Instant::now() + this.limit);
                Poll::Ready(item.map(|chunk| chunk.map_err(io::Error::other)))
            }
            Poll::Pending => match this.deadline.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    this.expired = true;
                    Poll::Ready(Some(Err(io::Error::new(
                        ErrorKind::TimedOut,
                        format!("upstream went quiet for {}s", this.limit.as_secs()),
                    ))))
                }
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future;

    use futures_util::{StreamExt, stream};

    use super::*;

    fn chunks_every(gap: Duration, count: usize) -> impl Stream<Item = Result<Bytes, io::Error>> {
        stream::unfold(0, move |sent| async move {
            if sent == count {
                return None;
            }
            tokio::time::sleep(gap).await;
            Some((Ok(Bytes::from_static(b"x")), sent + 1))
        })
    }

    async fn read_all(body: Body) -> Vec<Result<Bytes, axum::Error>> {
        body.into_data_stream().collect().await
    }

    #[test]
    fn zero_seconds_means_off_and_saving_applies_at_once() {
        let timeout = IdleTimeout::new(0);
        assert!(timeout.limit().0.is_none());
        timeout.set(30);
        assert_eq!(timeout.limit().0, Some(Duration::from_secs(30)));
    }

    #[tokio::test(start_paused = true)]
    async fn a_stream_that_keeps_sending_is_never_cut() {
        let limit = IdleTimeout::new(5).limit();
        let read = read_all(limit.watch(chunks_every(Duration::from_secs(4), 10))).await;
        assert_eq!(read.len(), 10);
        assert!(read.iter().all(Result::is_ok));
    }

    #[tokio::test(start_paused = true)]
    async fn a_stream_that_goes_quiet_is_cut() {
        let limit = IdleTimeout::new(5).limit();
        let quiet = chunks_every(Duration::from_secs(1), 1).chain(stream::pending());
        let read = read_all(limit.watch(quiet)).await;
        assert_eq!(read.len(), 2);
        assert!(read[0].is_ok());
        assert!(read[1].is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn waiting_with_no_answer_gives_a_gateway_timeout() {
        let limit = IdleTimeout::new(5).limit();
        let error = limit.wait(future::pending::<()>()).await.unwrap_err();
        assert_eq!(error.status, StatusCode::GATEWAY_TIMEOUT);
    }

    #[tokio::test(start_paused = true)]
    async fn when_off_nothing_is_ever_cut() {
        let limit = IdleTimeout::new(0).limit();
        let slow = async {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            7
        };
        assert_eq!(limit.wait(slow).await.unwrap(), 7);
        let read = read_all(limit.watch(chunks_every(Duration::from_secs(600), 2))).await;
        assert_eq!(read.len(), 2);
    }
}
