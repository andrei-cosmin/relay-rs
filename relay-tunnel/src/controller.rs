use std::sync::Arc;

use axum::{Extension, Router};
use dioxus_fullstack::{JsonEncoding, Streaming};
use relay_core::{Controller, Resources};

use crate::{TunnelApi, TunnelState};

#[derive(Clone)]
pub struct TunnelController {
    state: Arc<TunnelState>,
}

impl Controller for TunnelController {
    fn build(resources: &Resources) -> Self {
        Self { state: resources.get::<TunnelState>() }
    }

    fn expose(&self, router: Router) -> Router {
        let api: Arc<dyn TunnelApi> = Arc::new(self.clone());
        router.layer(Extension(api))
    }
}

impl TunnelApi for TunnelController {
    fn url(&self) -> String {
        self.state.url()
    }

    fn enabled(&self) -> bool {
        self.state.mode().enabled()
    }

    fn restart(&self) {
        self.state.request_restart();
    }

    fn watch(&self) -> Streaming<String, JsonEncoding> {
        Streaming::new(self.state.watch())
    }
}
