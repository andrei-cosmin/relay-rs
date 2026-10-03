use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use haze::{Pack, Seq};
use http_body::Body as _;
use relay_core::{Body, Ctx, Error, Incoming, Interceptor, Outgoing, Reply, Target};
use relay_targets::TargetStore;
use relay_vpn::Vpns;
use reqwest::Client;

use crate::{Sequence, transport::Transport};

#[derive(Clone, Pack)]
pub struct Proxy {
    targets: TargetStore,
    vpns: Vpns,
    interceptors: Seq<dyn Interceptor>,
    ids: Sequence,
    #[pack(func = Transport::default())]
    transport: Transport,
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

    pub async fn handle(&self, request: Request) -> Reply {
        let (parts, mut body) = request.into_parts();
        let (target, url) = {
            let targets = self.targets.read();
            let Some(target) = Self::pick_target(&targets.targets, parts.uri.path()) else {
                return (StatusCode::NOT_FOUND, "no target for this path").into_response();
            };
            (
                target.clone(),
                Self::url(target, parts.uri.path(), parts.uri.query()),
            )
        };
        let ctx = Ctx {
            id: self.ids.next(),
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
        for interceptor in &self.interceptors {
            if let Err(reply) = interceptor.on_request(&ctx, &mut outgoing) {
                return self.finish(&ctx, Incoming::from_reply(*reply));
            }
        }
        let socket = match &ctx.target.vpn {
            Some(name) => match self.vpns.socks_address(name) {
                Some(socket) => Some(socket),
                None => {
                    let error = Error {
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        message: format!("vpn {name} is down"),
                    };
                    return self.finish(&ctx, Incoming::from_error(error));
                }
            },
            None => None,
        };
        for interceptor in &self.interceptors {
            body = interceptor.tap_request(&ctx, body);
        }
        let incoming = Self::send(&self.transport.client(socket), outgoing, body)
            .await
            .unwrap_or_else(Incoming::from_error);
        self.finish(&ctx, incoming)
    }

    fn finish(&self, ctx: &Ctx, mut incoming: Incoming) -> Reply {
        for interceptor in &self.interceptors {
            if let Err(reply) = interceptor.on_response(ctx, &mut incoming) {
                incoming = Incoming::from_reply(*reply);
                break;
            }
        }
        for interceptor in &self.interceptors {
            incoming.body = interceptor.tap_response(ctx, incoming.body);
        }
        incoming.into_reply()
    }

    async fn send(http: &Client, outgoing: Outgoing, body: Body) -> Result<Incoming, Error> {
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
