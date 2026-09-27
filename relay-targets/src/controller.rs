use std::sync::Arc;

use axum::{Extension, Router};
use relay_core::{Controller, Resources, TargetCache, TargetList};

use crate::TargetsApi;

#[derive(Clone)]
pub struct TargetsController {
    targets: Arc<TargetCache>,
}

impl Controller for TargetsController {
    fn build(resources: &Resources) -> Self {
        Self { targets: resources.get::<TargetCache>() }
    }

    fn expose(&self, router: Router) -> Router {
        let api: Arc<dyn TargetsApi> = Arc::new(self.clone());
        router.layer(Extension(api))
    }
}

impl TargetsApi for TargetsController {
    fn targets(&self) -> TargetList {
        self.targets.read().clone()
    }

    fn save(&self, targets: TargetList) -> Result<(), String> {
        self.targets.apply(targets).map_err(|error| error.message)
    }
}
