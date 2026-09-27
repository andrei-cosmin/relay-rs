use axum::Router;

#[derive(Clone)]
pub struct Endpoints(pub Router);
