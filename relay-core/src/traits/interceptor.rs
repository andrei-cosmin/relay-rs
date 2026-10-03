use crate::{Body, Ctx, Incoming, Outgoing, Reply};

pub trait Interceptor: Send + Sync {
    fn on_request(&self, _ctx: &Ctx, _outgoing: &mut Outgoing) -> Result<(), Box<Reply>> {
        Ok(())
    }

    fn on_response(&self, _ctx: &Ctx, _incoming: &mut Incoming) -> Result<(), Box<Reply>> {
        Ok(())
    }

    fn tap_request(&self, _ctx: &Ctx, body: Body) -> Body {
        body
    }

    fn tap_response(&self, _ctx: &Ctx, body: Body) -> Body {
        body
    }
}
