use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use reqwest::{Client, redirect};

#[derive(Clone)]
pub(crate) struct Transport {
    direct: Client,
    through_vpn: Arc<RwLock<HashMap<SocketAddr, Client>>>,
}

impl Transport {
    pub(crate) fn client(&self, socket: Option<SocketAddr>) -> Client {
        let Some(socket) = socket else {
            return self.direct.clone();
        };
        if let Some(client) = self
            .through_vpn
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&socket)
        {
            return client.clone();
        }
        let proxy =
            reqwest::Proxy::all(format!("socks5h://{socket}")).expect("a socks5h proxy url");
        let client = Self::builder()
            .proxy(proxy)
            .build()
            .expect("building the vpn http client");
        self.through_vpn
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(socket)
            .or_insert(client)
            .clone()
    }

    fn builder() -> reqwest::ClientBuilder {
        Client::builder().redirect(redirect::Policy::none())
    }
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            direct: Self::builder().build().expect("building the http client"),
            through_vpn: Arc::default(),
        }
    }
}
