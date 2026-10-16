use serde::{Serialize, de::DeserializeOwned};

pub trait Record: Serialize + DeserializeOwned {
    const TABLE: &'static str;
    const VERSION: u32;

    fn key(&self) -> u64;

    fn table() -> String {
        format!("{}/{}", Self::TABLE, Self::VERSION)
    }
}
