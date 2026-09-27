use std::sync::Arc;

use axum::{Extension, Router};
use relay_core::{Controller, Resources};

use crate::{VpnApi, VpnInfo, VpnState};

#[derive(Clone)]
pub struct VpnController {
    state: Arc<VpnState>,
}

impl Controller for VpnController {
    fn build(resources: &Resources) -> Self {
        Self { state: resources.get::<VpnState>() }
    }

    fn expose(&self, router: Router) -> Router {
        let api: Arc<dyn VpnApi> = Arc::new(self.clone());
        router.layer(Extension(api))
    }
}

impl VpnApi for VpnController {
    fn list(&self) -> Vec<VpnInfo> {
        self.state.list()
    }

    fn add(&self, name: String, config: String) -> Result<(), String> {
        self.state.add(name, config)
    }

    fn remove(&self, name: String) -> Result<(), String> {
        self.state.remove(name)
    }
}
