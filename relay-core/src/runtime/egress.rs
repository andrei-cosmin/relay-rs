use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub struct Egress {
    exits: RwLock<HashMap<String, SocketAddr>>,
}

impl Egress {
    pub fn new() -> Self {
        Self { exits: RwLock::new(HashMap::new()) }
    }

    pub fn set(&self, name: &str, socks: SocketAddr) {
        self.write().insert(name.to_owned(), socks);
    }

    pub fn remove(&self, name: &str) {
        self.write().remove(name);
    }

    pub fn get(&self, name: &str) -> Option<SocketAddr> {
        self.read().get(name).copied()
    }

    fn read(&self) -> RwLockReadGuard<'_, HashMap<String, SocketAddr>> {
        self.exits.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, HashMap<String, SocketAddr>> {
        self.exits.write().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
