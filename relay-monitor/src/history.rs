use serde::{Deserialize, Serialize};

use crate::Entry;

#[derive(Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct History {
    pub page: usize,
    pub size: usize,
    pub total: u64,
    pub entries: Vec<Entry>,
}

impl History {
    pub fn pages(&self) -> usize {
        if self.size == 0 {
            return 1;
        }
        (self.total as usize).div_ceil(self.size).max(1)
    }
}
