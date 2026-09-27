use std::{net::SocketAddr, path::PathBuf};

use axum::{
    Router,
    http::{HeaderValue, header::CACHE_CONTROL},
};
use relay_core::{Endpoints, Error, Resources, Service};
use tokio::net::TcpListener;
use tower_http::{services::ServeFile, set_header::SetResponseHeaderLayer};

use crate::Serve;

pub struct Admin {
    address: SocketAddr,
    index: PathBuf,
    router: Router,
}

impl Admin {
    fn address(configured: SocketAddr) -> SocketAddr {
        if dioxus_cli_config::is_cli_enabled() {
            return dioxus_cli_config::fullstack_address_or_localhost();
        }
        configured
    }

    fn index() -> PathBuf {
        let public = match std::env::var_os("DIOXUS_PUBLIC_PATH") {
            Some(path) => PathBuf::from(path),
            None => std::env::current_exe().ok().and_then(|executable| executable.parent().map(|folder| folder.join("public"))).unwrap_or_else(|| PathBuf::from("public")),
        };
        public.join("index.html")
    }
}

#[async_trait::async_trait]
impl Service for Admin {
    fn build(resources: &Resources) -> Self {
        Self { address: Self::address(resources.get::<Serve>().0), index: Self::index(), router: resources.get::<Endpoints>().0.clone() }
    }

    async fn run(&self) -> Result<(), Error> {
        let address = self.address;
        let listener = TcpListener::bind(address).await.map_err(|error| Error::internal(format!("{address}: {error}")))?;
        let router = self
            .router
            .clone()
            .fallback_service(ServeFile::new(&self.index))
            .layer(SetResponseHeaderLayer::if_not_present(CACHE_CONTROL, HeaderValue::from_static("no-cache")));
        axum::serve(listener, router)
            .with_graceful_shutdown(Self::until_shutdown())
            .await
            .map_err(|error| Error::internal(error.to_string()))
    }
}
