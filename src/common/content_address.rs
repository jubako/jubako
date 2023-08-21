use crate::bases::*;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct ContentAddress {
    pub pack_id: PackId,
    pub content_id: ContentIdx,
}

impl ContentAddress {
    pub fn new(pack_id: PackId, content_id: ContentIdx) -> Self {
        Self {
            pack_id,
            content_id,
        }
    }
}
