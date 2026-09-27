use std::sync::Arc;

use axum::Router;
use dioxus_server::{DioxusRouterExt, FullstackState};
use relay_core::{Controller, Endpoints, Error, Leaf, Leaves, Resources, Service};

pub struct Manager {
    resources: Resources,
    leaves: Vec<Arc<dyn Leaf>>,
    router: Router,
    services: Vec<Arc<dyn Service>>,
}

impl Manager {
    pub fn new(resources: Resources) -> Self {
        let router = Router::<FullstackState>::new().register_server_functions().serve_static_assets().with_state(FullstackState::headless());
        Self { resources, leaves: Vec::new(), router, services: Vec::new() }
    }

    pub fn leaf<L: Leaf + 'static>(&mut self) {
        self.leaves.push(Arc::new(L::build(&self.resources)));
    }

    pub fn controller<C: Controller + 'static>(&mut self) {
        let controller = C::build(&self.resources);
        self.router = controller.expose(std::mem::take(&mut self.router));
    }

    pub fn wire(&mut self) {
        self.resources.insert(Leaves(self.leaves.clone()));
        self.resources.insert(Endpoints(self.router.clone()));
    }

    pub fn service<S: Service + 'static>(&mut self) {
        self.services.push(Arc::new(S::build(&self.resources)));
    }

    pub async fn run(self) -> Result<(), Error> {
        let mut runs = Vec::new();
        for service in &self.services {
            runs.push(service.run());
        }
        futures_util::future::try_join_all(runs).await.map(|_| ())
    }
}
