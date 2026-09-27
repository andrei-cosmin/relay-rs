use std::sync::{RwLock, RwLockReadGuard};

use crate::{Config, Error, TargetList};

pub struct TargetCache {
    inner: RwLock<TargetList>,
}

impl TargetCache {
    pub fn load() -> Result<Self, Error> {
        Ok(Self { inner: RwLock::new(TargetList::load()?) })
    }

    pub fn read(&self) -> RwLockReadGuard<'_, TargetList> {
        self.inner.read().expect("targets lock")
    }

    pub fn apply(&self, fresh: TargetList) -> Result<(), Error> {
        let mut targets = self.inner.write().expect("targets lock");
        fresh.save()?;
        *targets = fresh;
        Ok(())
    }
}

impl Config for TargetList {
    const FILE: &'static str = "targets.ron";
}
