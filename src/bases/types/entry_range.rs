use super::{EntryCount, EntryIdx};

#[derive(Debug, PartialEq, Eq)]
pub struct EntryRange {
    pub offset: EntryIdx,
    pub count: EntryCount,
}

impl EntryRange {
    pub fn new(offset: EntryIdx, count: EntryCount) -> Self {
        Self { offset, count }
    }
}
