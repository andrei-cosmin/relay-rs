use std::sync::Arc;

use relay_core::{Body, Ctx, Incoming, Leaf, Outgoing, Reply, Resources};

use crate::{ByteMeter, Direction, Entry, EntryLog};

pub struct Monitor {
    log: Arc<EntryLog>,
}

impl Leaf for Monitor {
    fn build(resources: &Resources) -> Self {
        Self { log: resources.get::<EntryLog>() }
    }

    fn on_request(&self, ctx: &Ctx, _outgoing: &mut Outgoing) -> Result<(), Reply> {
        self.log.push(Entry::received(ctx));
        Ok(())
    }

    fn on_response(&self, ctx: &Ctx, incoming: &mut Incoming) -> Result<(), Reply> {
        let status = incoming.status.as_u16();
        self.log.update(ctx.id, |entry| entry.answered(status));
        Ok(())
    }

    fn tap_request(&self, ctx: &Ctx, body: Body) -> Body {
        ByteMeter::wrap(body, self.log.clone(), ctx.id, Direction::Sent)
    }

    fn tap_response(&self, ctx: &Ctx, body: Body) -> Body {
        ByteMeter::wrap(body, self.log.clone(), ctx.id, Direction::Received)
    }
}
