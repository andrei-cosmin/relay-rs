#[cfg(feature = "backend")]
mod filestore;
mod page;
#[cfg(feature = "backend")]
mod rdbstore;

#[cfg(feature = "backend")]
pub use filestore::FileStore;
pub use page::Page;
#[cfg(feature = "backend")]
pub use rdbstore::{RdbStore, Record};
