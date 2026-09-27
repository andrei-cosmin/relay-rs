use futures_util::{Stream, StreamExt};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::Shutdown;

pub struct Feed<T> {
    sender: broadcast::Sender<T>,
}

impl<T: Clone + Send + 'static> Feed<T> {
    pub fn new(capacity: usize) -> Self {
        Self { sender: broadcast::channel(capacity).0 }
    }

    pub fn push(&self, item: T) {
        let _ = self.sender.send(item);
    }

    pub fn subscribe(&self) -> impl Stream<Item=T> + Send + 'static {
        BroadcastStream::new(self.sender.subscribe()).filter_map(|item| async move { item.ok() }).take_until(Shutdown::requested())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subscribers_only_see_pushes_after_they_join() {
        let feed = Feed::new(4);
        feed.push(1);
        let mut stream = std::pin::pin!(feed.subscribe());
        feed.push(2);
        feed.push(3);
        assert_eq!(stream.next().await, Some(2));
        assert_eq!(stream.next().await, Some(3));
    }

    #[test]
    fn pushing_with_nobody_listening_is_fine() {
        Feed::new(1).push("lost");
    }
}
