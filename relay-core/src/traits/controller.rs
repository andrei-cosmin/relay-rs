use axum::Router;

use crate::Resources;

pub trait Controller: Send + Sync {
    fn build(resources: &Resources) -> Self
    where
        Self: Sized;

    fn expose(&self, router: Router) -> Router;
}
