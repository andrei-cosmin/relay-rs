use axum::{
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
};
use haze::Pack;
use relay_core::{Ctx, Incoming, Interceptor, Outgoing, Reply, Rule};

#[derive(Clone, Pack)]
pub struct Rules {}

#[haze::register(order = 10)]
impl Interceptor for Rules {
    fn on_request(&self, ctx: &Ctx, outgoing: &mut Outgoing) -> Result<(), Box<Reply>> {
        for rule in &ctx.target.rules {
            self.apply_to_request(rule, ctx, outgoing)?;
        }
        Ok(())
    }

    fn on_response(&self, ctx: &Ctx, incoming: &mut Incoming) -> Result<(), Box<Reply>> {
        for rule in &ctx.target.rules {
            self.apply_to_reply(rule, incoming);
        }
        Ok(())
    }
}

impl Rules {
    fn apply_to_request(
        &self,
        rule: &Rule,
        ctx: &Ctx,
        outgoing: &mut Outgoing,
    ) -> Result<(), Box<Reply>> {
        match rule {
            Rule::Block => Err(Box::new((StatusCode::FORBIDDEN, "blocked").into_response())),
            Rule::Cors if ctx.method == Method::OPTIONS => {
                let mut reply = StatusCode::NO_CONTENT.into_response();
                reply
                    .headers_mut()
                    .insert("access-control-allow-origin", HeaderValue::from_static("*"));
                reply.headers_mut().insert(
                    "access-control-allow-methods",
                    HeaderValue::from_static("*"),
                );
                let allowed = ctx
                    .headers
                    .get("access-control-request-headers")
                    .cloned()
                    .unwrap_or(HeaderValue::from_static("*"));
                reply
                    .headers_mut()
                    .insert("access-control-allow-headers", allowed);
                reply
                    .headers_mut()
                    .insert("access-control-max-age", HeaderValue::from_static("86400"));
                Err(Box::new(reply))
            }
            Rule::Cors => Ok(()),
            Rule::Key(_) if ctx.method == Method::OPTIONS => Ok(()),
            Rule::Key(secret) if Self::consume(secret, ctx, outgoing) => Ok(()),
            Rule::Key(_) => Err(Box::new(
                (StatusCode::UNAUTHORIZED, "wrong key").into_response(),
            )),
            Rule::Set(name, value) => {
                outgoing.headers.insert(name.clone(), value.clone());
                Ok(())
            }
            Rule::Remove(patterns) => {
                while let Some(name) = outgoing
                    .headers
                    .keys()
                    .find(|name| Self::matches(patterns, name.as_str()))
                    .cloned()
                {
                    outgoing.headers.remove(name);
                }
                Ok(())
            }
        }
    }

    fn apply_to_reply(&self, rule: &Rule, incoming: &mut Incoming) {
        if let Rule::Cors = rule {
            incoming
                .headers
                .insert("access-control-allow-origin", HeaderValue::from_static("*"));
            incoming.headers.insert(
                "access-control-expose-headers",
                HeaderValue::from_static("*"),
            );
        }
    }

    fn consume(secret: &str, ctx: &Ctx, outgoing: &mut Outgoing) -> bool {
        let bearer = format!("Bearer {secret}");
        let mut presented = false;
        for name in ["authorization", "x-api-key"] {
            if let Some(Ok(value)) = ctx.headers.get(name).map(|value| value.to_str())
                && (value == secret || value == bearer)
            {
                presented = true;
                if outgoing.headers.get(name).is_some_and(|sent| sent == value) {
                    outgoing.headers.remove(name);
                }
            }
        }
        presented
    }

    fn matches(patterns: &[String], name: &str) -> bool {
        patterns
            .iter()
            .any(|pattern| match pattern.strip_suffix('*') {
                Some(prefix) => name.starts_with(prefix),
                None => name == pattern,
            })
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderName};
    use relay_core::{Body, Target};

    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    fn ctx(method: Method, pairs: &[(&str, &str)]) -> Ctx {
        let target = Target {
            prefix: "/sm".into(),
            upstream: "http://upstream".into(),
            rules: Vec::new(),
            vpn: None,
        };
        Ctx {
            id: 1,
            method,
            path: "/sm/x".into(),
            headers: headers(pairs),
            target,
        }
    }

    fn outgoing(pairs: &[(&str, &str)]) -> Outgoing {
        Outgoing {
            method: Method::POST,
            url: "http://upstream/x".into(),
            headers: headers(pairs),
        }
    }

    fn rule(line: &str) -> Rule {
        Rule::try_from(line.to_owned()).unwrap()
    }

    fn status(result: Result<(), Box<Reply>>) -> u16 {
        result.unwrap_err().status().as_u16()
    }

    #[test]
    fn set_overwrites_the_header() {
        let mut out = outgoing(&[("authorization", "old")]);
        Rules {}
            .apply_to_request(
                &rule("set authorization new"),
                &ctx(Method::POST, &[]),
                &mut out,
            )
            .unwrap();
        assert_eq!(out.headers["authorization"], "new");
    }

    #[test]
    fn remove_takes_names_and_globs() {
        let mut out = outgoing(&[
            ("origin", "a"),
            ("cf-ray", "b"),
            ("cf-connecting-ip", "c"),
            ("accept", "d"),
        ]);
        Rules {}
            .apply_to_request(
                &rule("remove origin cf-*"),
                &ctx(Method::POST, &[]),
                &mut out,
            )
            .unwrap();
        assert_eq!(
            out.headers
                .keys()
                .map(|name| name.as_str())
                .collect::<Vec<_>>(),
            ["accept"]
        );
    }

    #[test]
    fn cors_answers_preflights_and_lets_the_rest_through() {
        let preflight = ctx(
            Method::OPTIONS,
            &[(
                "access-control-request-headers",
                "authorization, content-type",
            )],
        );
        let reply = Rules {}
            .apply_to_request(&rule("cors"), &preflight, &mut outgoing(&[]))
            .unwrap_err();
        assert_eq!(reply.status().as_u16(), 204);
        assert_eq!(
            reply.headers()["access-control-allow-headers"],
            "authorization, content-type"
        );
        assert!(
            Rules {}
                .apply_to_request(&rule("cors"), &ctx(Method::POST, &[]), &mut outgoing(&[]))
                .is_ok()
        );
    }

    #[test]
    fn cors_marks_replies() {
        let mut reply = Incoming {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Body::empty(),
        };
        Rules {}.apply_to_reply(&rule("cors"), &mut reply);
        assert_eq!(reply.headers["access-control-allow-origin"], "*");
    }

    #[test]
    fn block_forbids() {
        assert_eq!(
            status(Rules {}.apply_to_request(
                &rule("block"),
                &ctx(Method::GET, &[]),
                &mut outgoing(&[])
            )),
            403
        );
    }

    #[test]
    fn key_accepts_bearer_or_x_api_key_and_strips_it() {
        let mut bearer = outgoing(&[("authorization", "Bearer hunter2")]);
        Rules {}
            .apply_to_request(
                &rule("key hunter2"),
                &ctx(Method::POST, &[("authorization", "Bearer hunter2")]),
                &mut bearer,
            )
            .unwrap();
        assert!(bearer.headers.get("authorization").is_none());
        let mut api_key = outgoing(&[("x-api-key", "hunter2")]);
        Rules {}
            .apply_to_request(
                &rule("key hunter2"),
                &ctx(Method::POST, &[("x-api-key", "hunter2")]),
                &mut api_key,
            )
            .unwrap();
        assert!(api_key.headers.get("x-api-key").is_none());
    }

    #[test]
    fn key_works_in_any_order_with_set_authorization() {
        let request = ctx(Method::POST, &[("authorization", "Bearer hunter2")]);
        let mut out = outgoing(&[("authorization", "Bearer hunter2")]);
        Rules {}
            .apply_to_request(&rule("set authorization Bearer real"), &request, &mut out)
            .unwrap();
        Rules {}
            .apply_to_request(&rule("key hunter2"), &request, &mut out)
            .unwrap();
        assert_eq!(out.headers["authorization"], "Bearer real");
    }

    #[test]
    fn key_rejects_wrong_or_missing_secrets_but_not_preflights() {
        assert_eq!(
            status(Rules {}.apply_to_request(
                &rule("key hunter2"),
                &ctx(Method::POST, &[("authorization", "Bearer nope")]),
                &mut outgoing(&[("authorization", "Bearer nope")])
            )),
            401
        );
        assert_eq!(
            status(Rules {}.apply_to_request(
                &rule("key hunter2"),
                &ctx(Method::POST, &[]),
                &mut outgoing(&[])
            )),
            401
        );
        assert!(
            Rules {}
                .apply_to_request(
                    &rule("key hunter2"),
                    &ctx(Method::OPTIONS, &[]),
                    &mut outgoing(&[])
                )
                .is_ok()
        );
    }
}
