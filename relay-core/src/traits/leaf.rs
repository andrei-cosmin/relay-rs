use crate::{Body, Ctx, Incoming, Outgoing, Reply, Resources};

pub trait Leaf: Send + Sync {
    fn build(resources: &Resources) -> Self
    where
        Self: Sized;

    fn on_request(&self, _ctx: &Ctx, _outgoing: &mut Outgoing) -> Result<(), Reply> {
        Ok(())
    }

    fn on_response(&self, _ctx: &Ctx, _incoming: &mut Incoming) -> Result<(), Reply> {
        Ok(())
    }

    fn tap_request(&self, _ctx: &Ctx, body: Body) -> Body {
        body
    }

    fn tap_response(&self, _ctx: &Ctx, body: Body) -> Body {
        body
    }
}
