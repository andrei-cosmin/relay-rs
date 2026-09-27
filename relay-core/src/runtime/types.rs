use http::{HeaderMap, Method, StatusCode};

use crate::{Error, Target};

pub type Body = axum::body::Body;
pub type Reply = axum::response::Response;

pub struct Ctx {
    pub id: u64,
    pub method: Method,
    pub path: String,
    pub headers: HeaderMap,
    pub target: Target,
}

pub struct Outgoing {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
}

pub struct Incoming {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
}

impl Incoming {
    pub fn from_reply(reply: Reply) -> Self {
        let (parts, body) = reply.into_parts();
        Self { status: parts.status, headers: parts.headers, body }
    }

    pub fn from_error(error: Error) -> Self {
        Self { status: error.status, headers: HeaderMap::new(), body: Body::from(error.message) }
    }

    pub fn into_reply(self) -> Reply {
        let mut reply = Reply::new(self.body);
        *reply.status_mut() = self.status;
        *reply.headers_mut() = self.headers;
        reply
    }
}
