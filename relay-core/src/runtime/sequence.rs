use std::sync::atomic::{AtomicU64, Ordering};

pub struct Sequence {
    next: AtomicU64,
}

impl Sequence {
    pub fn after(last: Option<u64>) -> Self {
        Self { next: AtomicU64::new(last.map_or(1, |last| last + 1)) }
    }

    pub fn next(&self) -> u64 {
        self.next.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continues_after_the_last_key_or_starts_at_one() {
        let sequence = Sequence::after(Some(41));
        assert_eq!((sequence.next(), sequence.next()), (42, 43));
        assert_eq!(Sequence::after(None).next(), 1);
    }
}
