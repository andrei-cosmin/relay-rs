#[cfg(feature = "server")]
mod filestore;
mod page;
#[cfg(feature = "server")]
mod rdbstore;

#[cfg(feature = "server")]
pub use filestore::FileStore;
pub use page::Page;
#[cfg(feature = "server")]
pub use rdbstore::{RdbStore, Record};
