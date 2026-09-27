use std::sync::Arc;

use crate::Leaf;

#[derive(Clone)]
pub struct Leaves(pub Vec<Arc<dyn Leaf>>);
