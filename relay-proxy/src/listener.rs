use std::net::SocketAddr;

use axum::{
    Router,
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
};
use haze::Later;
use relay_core::{Error, Reply};
use tokio::net::TcpListener;

use crate::{Listen, Proxy};

#[derive(Clone, Copy)]
pub struct Listener {
    address: SocketAddr,
}

#[haze::resource]
async fn listener(listen: Listen, proxy: Later<Proxy>) -> Result<Listener, Error> {
    Listener::bind(listen, proxy).await
}

impl Listener {
    async fn bind(listen: Listen, proxy: Later<Proxy>) -> Result<Self, Error> {
        let socket = TcpListener::bind(listen.0)
            .await
            .map_err(|error| Error::internal(format!("{}: {error}", listen.0)))?;
        let address = socket
            .local_addr()
            .map_err(|error| Error::internal(error.to_string()))?;
        let router = Router::new().fallback(Self::handle).with_state(proxy);
        tokio::spawn(async move {
            if let Err(error) = axum::serve(socket, router).await {
                tracing::error!("proxy: {error}");
            }
        });
        Ok(Self { address })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    async fn handle(State(proxy): State<Later<Proxy>>, request: Request) -> Reply {
        match proxy.get() {
            Ok(proxy) => proxy.handle(request).await,
            Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "the relay is starting").into_response(),
        }
    }
}
