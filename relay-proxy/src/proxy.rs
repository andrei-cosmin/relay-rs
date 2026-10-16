use std::mem;

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

use crate::{IdleTimeout, Sequence, cancel::CancelGuard, idle::IdleLimit, transport::Transport};

#[derive(Clone, Pack)]
pub struct Proxy {
    targets: TargetStore,
    vpns: Vpns,
    interceptors: Seq<dyn Interceptor>,
    ids: Sequence,
    idle_timeout: IdleTimeout,
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
        let (parts, body) = request.into_parts();
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
        let outgoing = Outgoing {
            method: parts.method,
            url,
            headers: parts.headers,
        };
        let guard = CancelGuard::new(&self.interceptors, &ctx);
        let incoming = self.exchange(&ctx, outgoing, body).await;
        guard.disarm();
        self.finish(&ctx, incoming)
    }

    async fn exchange(&self, ctx: &Ctx, mut outgoing: Outgoing, mut body: Body) -> Incoming {
        for interceptor in &self.interceptors {
            if let Err(reply) = interceptor.on_request(ctx, &mut outgoing) {
                return Incoming::from_reply(*reply);
            }
        }
        let socket = match &ctx.target.vpn {
            Some(name) => match self.vpns.socks_address(name) {
                Some(socket) => Some(socket),
                None => {
                    return Incoming::from_error(Error {
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        message: format!("vpn {name} is down"),
                    });
                }
            },
            None => None,
        };
        for interceptor in &self.interceptors {
            body = interceptor.tap_request(ctx, body);
        }
        let limit = self.idle_timeout.limit();
        Self::send(&self.transport.client(socket), outgoing, body, limit)
            .await
            .unwrap_or_else(Incoming::from_error)
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

    async fn send(
        http: &Client,
        outgoing: Outgoing,
        body: Body,
        limit: IdleLimit,
    ) -> Result<Incoming, Error> {
        let mut request = http
            .request(outgoing.method, outgoing.url)
            .headers(Self::clean(outgoing.headers));
        if !body.is_end_stream() {
            request = request.body(reqwest::Body::wrap_stream(body.into_data_stream()));
        }
        let mut response = limit.wait(request.send()).await?.map_err(|error| Error {
            status: StatusCode::BAD_GATEWAY,
            message: format!("upstream failed: {error}"),
        })?;
        let status = response.status();
        let headers = Self::clean(mem::take(response.headers_mut()));
        Ok(Incoming {
            status,
            headers,
            body: limit.watch(response.bytes_stream()),
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
            || path
                .strip_prefix(target.prefix.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    }

    fn url(target: &Target, path: &str, query: Option<&str>) -> String {
        let rest = if target.prefix == "/" {
            path
        } else {
            &path[target.prefix.len()..]
        };
        let base = target.upstream.trim_end_matches('/');
        match query {
            Some(query) => format!("{base}{rest}?{query}"),
            None => format!("{base}{rest}"),
        }
    }

    fn clean(mut headers: HeaderMap) -> HeaderMap {
        for name in Self::HOP_BY_HOP {
            headers.remove(name);
        }
        headers
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
