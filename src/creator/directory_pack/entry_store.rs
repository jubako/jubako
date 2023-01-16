use super::{layout, BasicEntry, Entry, WritableTell};
use crate::bases::*;
use crate::common;

pub struct EntryStore {
    idx: EntryStoreIdx,
    entries: Vec<Box<dyn Entry>>,
    layout: layout::Entry,
}

impl EntryStore {
    pub fn new(idx: EntryStoreIdx, layout: layout::Entry) -> Self {
        Self {
            idx,
            entries: vec![],
            layout,
        }
    }

    pub fn add_entry(&mut self, variant_id: Option<VariantIdx>, values: Vec<common::Value>) {
        self.entries
            .push(Box::new(BasicEntry::new(variant_id, values)));
    }

    pub fn get_idx(&self) -> EntryStoreIdx {
        self.idx
    }

    pub(crate) fn finalize(&mut self) {
        for entry in &mut self.entries {
            entry.finalize(&mut self.layout);
        }
        self.layout.finalize();
    }
}

impl WritableTell for EntryStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for entry in &self.entries {
            self.layout.write_entry(entry.as_ref(), stream)?;
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
