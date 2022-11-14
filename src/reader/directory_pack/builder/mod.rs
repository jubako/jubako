use crate::bases::*;

pub trait BuilderTrait {
    type Entry;
    fn create_entry(&self, idx: EntryIdx, reader: &Reader) -> Result<Self::Entry>;
}
