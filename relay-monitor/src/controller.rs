use std::sync::Arc;

use axum::{Extension, Router};
use dioxus_fullstack::{JsonEncoding, Streaming};
use relay_core::{Controller, Resources};

use crate::{Entry, EntryLog, History, MonitorApi};

pub struct MonitorController {
    log: Arc<EntryLog>,
}

impl Controller for MonitorController {
    fn build(resources: &Resources) -> Self {
        Self { log: resources.get::<EntryLog>() }
    }

    fn expose(&self, router: Router) -> Router {
        let api: Arc<dyn MonitorApi> = Arc::new(Self { log: self.log.clone() });
        router.layer(Extension(api))
    }
}

impl MonitorApi for MonitorController {
    fn history(&self, page: usize) -> Result<History, String> {
        self.log.history(page).map_err(|error| error.message)
    }

    fn clear(&self) -> Result<(), String> {
        self.log.clear().map_err(|error| error.message)
    }

    fn watch(&self) -> Streaming<Entry, JsonEncoding> {
        Streaming::new(self.log.watch())
    }
}
