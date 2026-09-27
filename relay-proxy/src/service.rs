use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use http_body::Body as _;
use relay_core::{
    Body, Ctx, Egress, Error, Incoming, Leaf, Leaves, Outgoing, Reply, Resources, Sequence,
    Service, Target, TargetCache,
};
use tokio::net::TcpListener;

use crate::Listen;

pub struct Proxy {
    listen: SocketAddr,
    inner: Arc<Inner>,
}

struct Inner {
    targets: Arc<TargetCache>,
    leaves: Vec<Arc<dyn Leaf>>,
    http: reqwest::Client,
    egress: Arc<Egress>,
    clients: RwLock<HashMap<SocketAddr, reqwest::Client>>,
    ids: Arc<Sequence>,
}

impl Inner {
    fn client(&self, socket: Option<SocketAddr>) -> reqwest::Client {
        let Some(socket) = socket else {
            return self.http.clone();
        };
        if let Some(client) = self
            .clients
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&socket)
        {
            return client.clone();
        }
        let proxy =
            reqwest::Proxy::all(format!("socks5h://{socket}")).expect("a socks5h proxy url");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .proxy(proxy)
            .build()
            .expect("building the vpn http client");
        self.clients
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(socket)
            .or_insert(client)
            .clone()
    }
}

impl Proxy {
    const HOP_BY_HOP: [&str; 8] = [
        "connection",
        "keep-alive",
        "proxy-connection",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
        "host",
    ];

    async fn handle(State(inner): State<Arc<Inner>>, request: Request) -> Reply {
        let (parts, mut body) = request.into_parts();
        let (target, url) = {
            let targets = inner.targets.read();
            let Some(target) = Self::pick_target(&targets.targets, parts.uri.path()) else {
                return (StatusCode::NOT_FOUND, "no target for this path").into_response();
            };
            (
                target.clone(),
                Self::url(target, parts.uri.path(), parts.uri.query()),
            )
        };
        let id = inner.ids.next();
        let ctx = Ctx {
            id,
            method: parts.method.clone(),
            path: parts.uri.path().to_owned(),
            headers: parts.headers.clone(),
            target,
        };
        let mut outgoing = Outgoing {
            method: parts.method,
            url,
            headers: parts.headers,
        };
        for leaf in &inner.leaves {
            if let Err(reply) = leaf.on_request(&ctx, &mut outgoing) {
                return Self::finish(&inner, &ctx, Incoming::from_reply(reply));
            }
        }
        let socket = match &ctx.target.vpn {
            Some(name) => match inner.egress.get(name) {
                Some(socket) => Some(socket),
                None => {
                    let error = Error {
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        message: format!("vpn {name} is down"),
                    };
                    return Self::finish(&inner, &ctx, Incoming::from_error(error));
                }
            },
            None => None,
        };
        for leaf in &inner.leaves {
            body = leaf.tap_request(&ctx, body);
        }
        let incoming = Self::send(&inner.client(socket), outgoing, body)
            .await
            .unwrap_or_else(|error| Incoming::from_error(error));
        Self::finish(&inner, &ctx, incoming)
    }

    fn finish(inner: &Inner, ctx: &Ctx, mut incoming: Incoming) -> Reply {
        for leaf in &inner.leaves {
            if let Err(reply) = leaf.on_response(ctx, &mut incoming) {
                incoming = Incoming::from_reply(reply);
                break;
            }
        }
        for leaf in &inner.leaves {
            incoming.body = leaf.tap_response(ctx, incoming.body);
        }
        incoming.into_reply()
    }

    async fn send(
        http: &reqwest::Client,
        outgoing: Outgoing,
        body: Body,
    ) -> Result<Incoming, Error> {
        let mut request = http
            .request(outgoing.method, outgoing.url)
            .headers(Self::clean(&outgoing.headers));
        if !body.is_end_stream() {
            request = request.body(reqwest::Body::wrap_stream(body.into_data_stream()));
        }
        let response = request.send().await.map_err(|error| Error {
            status: StatusCode::BAD_GATEWAY,
            message: format!("upstream failed: {error}"),
        })?;
        let status = response.status();
        let headers = Self::clean(response.headers());
        Ok(Incoming {
            status,
            headers,
            body: Body::from_stream(response.bytes_stream()),
        })
    }

    fn pick_target<'a>(targets: &'a [Target], path: &str) -> Option<&'a Target> {
        let mut best = None;
        let mut longest = 0;
        for target in targets {
            if Self::matches(target, path) && target.prefix.len() >= longest {
                best = Some(target);
                longest = target.prefix.len();
            }
        }
        best
    }

    fn matches(target: &Target, path: &str) -> bool {
        target.prefix == "/"
            || path == target.prefix
            || path.starts_with(&format!("{}/", target.prefix))
    }

    fn url(target: &Target, path: &str, query: Option<&str>) -> String {
        let rest = if target.prefix == "/" {
            path
        } else {
            &path[target.prefix.len()..]
        };
        let query = query.map(|query| format!("?{query}")).unwrap_or_default();
        format!("{}{rest}{query}", target.upstream.trim_end_matches('/'))
    }

    fn clean(headers: &HeaderMap) -> HeaderMap {
        let mut clean = headers.clone();
        for name in Self::HOP_BY_HOP {
            clean.remove(name);
        }
        clean
    }
}

#[async_trait::async_trait]
impl Service for Proxy {
    fn build(resources: &Resources) -> Self {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("building the http client");
        let inner = Inner {
            targets: resources.get::<TargetCache>(),
            leaves: resources.get::<Leaves>().0.clone(),
            http,
            egress: resources.get::<Egress>(),
            clients: RwLock::new(HashMap::new()),
            ids: resources.get::<Sequence>(),
        };
        Self {
            listen: resources.get::<Listen>().0,
            inner: Arc::new(inner),
        }
    }

    async fn run(&self) -> Result<(), Error> {
        let listen = self.listen;
        let listener = TcpListener::bind(listen)
            .await
            .map_err(|error| Error::internal(format!("{listen}: {error}")))?;
        let router = Router::new()
            .fallback(Self::handle)
            .with_state(self.inner.clone());
        axum::serve(listener, router)
            .with_graceful_shutdown(Self::until_shutdown())
            .await
            .map_err(|error| Error::internal(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(prefix: &str, upstream: &str) -> Target {
        Target {
            prefix: prefix.into(),
            upstream: upstream.into(),
            rules: Vec::new(),
            vpn: None,
        }
    }

    #[test]
    fn the_longest_matching_prefix_wins() {
        let targets = [
            target("/", "http://root"),
            target("/sm", "http://sm"),
            target("/sm/openai", "http://openai"),
        ];
        assert_eq!(
            Proxy::pick_target(&targets, "/sm/openai/v1")
                .unwrap()
                .upstream,
            "http://openai"
        );
        assert_eq!(
            Proxy::pick_target(&targets, "/sm/glm/v1").unwrap().upstream,
            "http://sm"
        );
        assert_eq!(
            Proxy::pick_target(&targets, "/smol").unwrap().upstream,
            "http://root"
        );
        assert!(Proxy::pick_target(&targets[1..], "/other").is_none());
    }

    #[test]
    fn a_prefix_matches_whole_segments_only() {
        let sm = target("/sm", "http://sm");
        assert!(Proxy::matches(&sm, "/sm"));
        assert!(Proxy::matches(&sm, "/sm/x"));
        assert!(!Proxy::matches(&sm, "/smol"));
    }

    #[test]
    fn the_url_drops_the_prefix_and_keeps_the_query() {
        assert_eq!(
            Proxy::url(&target("/sm", "http://sm/"), "/sm/v1/models", Some("a=1")),
            "http://sm/v1/models?a=1"
        );
        assert_eq!(
            Proxy::url(&target("/", "http://root"), "/v1/models", None),
            "http://root/v1/models"
        );
    }
}
