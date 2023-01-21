use super::private::WritableTell;
use super::{layout, EntryTrait};
use crate::bases::*;

pub struct EntryStore<Entry: EntryTrait> {
    idx: Delayed<EntryStoreIdx>,
    entries: Vec<Entry>,
    layout: layout::Entry,
}

impl<Entry: EntryTrait> EntryStore<Entry> {
    pub fn new(layout: layout::Entry) -> Self {
        Self {
            idx: Default::default(),
            entries: vec![],
            layout,
        }
    }

    pub fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn get_idx(&self) -> EntryStoreIdx {
        self.idx.get()
    }
}

pub trait EntryStoreTrait: WritableTell {
    fn set_idx(&mut self, idx: EntryStoreIdx);
    fn finalize(&mut self);
}

impl<Entry: EntryTrait> EntryStoreTrait for EntryStore<Entry> {
    fn finalize(&mut self) {
        for entry in &mut self.entries {
            entry.finalize(&mut self.layout);
        }
        self.layout.finalize();
    }

    fn set_idx(&mut self, idx: EntryStoreIdx) {
        self.idx.set(idx);
    }
}

impl<Entry: EntryTrait> WritableTell for EntryStore<Entry> {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for entry in &self.entries {
            self.layout.write_entry(entry, stream)?;
        }
        Ok(())
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        stream.write_u8(0x00)?; // kind
        self.layout.write(stream)?;
        stream.write_u64((self.entries.len() * self.layout.entry_size() as usize) as u64)?;
        Ok(())
    }
}
