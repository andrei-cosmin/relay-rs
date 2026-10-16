use haze::Seq;
use relay_core::{Ctx, Interceptor};

pub(crate) struct CancelGuard<'a> {
    interceptors: &'a Seq<dyn Interceptor>,
    ctx: &'a Ctx,
    armed: bool,
}

impl<'a> CancelGuard<'a> {
    pub(crate) fn new(interceptors: &'a Seq<dyn Interceptor>, ctx: &'a Ctx) -> Self {
        Self {
            interceptors,
            ctx,
            armed: true,
        }
    }

    pub(crate) fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for CancelGuard<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for interceptor in self.interceptors {
            interceptor.on_cancel(self.ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use axum::http::{HeaderMap, Method};
    use relay_core::Target;

    use super::*;

    struct Counter(Arc<AtomicUsize>);

    impl Interceptor for Counter {
        fn on_cancel(&self, _ctx: &Ctx) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn setup() -> (Seq<dyn Interceptor>, Ctx, Arc<AtomicUsize>) {
        let cancels = Arc::new(AtomicUsize::new(0));
        let interceptors = Seq::from(vec![
            Box::new(Counter(cancels.clone())) as Box<dyn Interceptor>
        ]);
        let ctx = Ctx {
            id: 1,
            method: Method::POST,
            path: "/sm/x".into(),
            headers: HeaderMap::new(),
            target: Target {
                prefix: "/sm".into(),
                upstream: "http://sm".into(),
                rules: Vec::new(),
                vpn: None,
            },
        };
        (interceptors, ctx, cancels)
    }

    #[test]
    fn dropping_an_armed_guard_cancels() {
        let (interceptors, ctx, cancels) = setup();
        drop(CancelGuard::new(&interceptors, &ctx));
        assert_eq!(cancels.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_disarmed_guard_stays_quiet() {
        let (interceptors, ctx, cancels) = setup();
        CancelGuard::new(&interceptors, &ctx).disarm();
        assert_eq!(cancels.load(Ordering::Relaxed), 0);
    }
}
