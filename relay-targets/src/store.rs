use std::sync::RwLockReadGuard;

use haze::Pack;
use relay_core::{DataDir, Error};
use relay_storage::FileStore;

use crate::TargetList;

#[derive(Clone, Pack)]
pub struct TargetStore {
    file: FileStore<TargetList>,
}

#[haze::resource]
fn targets_file(dir: DataDir) -> Result<FileStore<TargetList>, Error> {
    FileStore::open(dir.0.join("targets.ron"))
}

impl TargetStore {
    pub fn read(&self) -> RwLockReadGuard<'_, TargetList> {
        self.file.read()
    }

    pub fn save(&self, targets: TargetList) -> Result<(), Error> {
        self.file.set(targets)
    }
}
