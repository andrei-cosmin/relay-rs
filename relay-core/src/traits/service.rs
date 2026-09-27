use tokio::sync::Notify;

use crate::{Error, Resources};

static SHUTDOWN: Notify = Notify::const_new();

pub struct Shutdown;

impl Shutdown {
    pub fn request() {
        SHUTDOWN.notify_waiters();
    }

    pub async fn requested() {
        SHUTDOWN.notified().await;
    }
}

#[async_trait::async_trait]
pub trait Service: Send + Sync {
    fn build(resources: &Resources) -> Self
    where
        Self: Sized;

    async fn run(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn until_shutdown()
    where
        Self: Sized,
    {
        let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("sigterm handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
            _ = Shutdown::requested() => {}
        }
        Shutdown::request();
    }
}
