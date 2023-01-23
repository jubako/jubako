use super::private::WritableTell;
use super::schema;
use super::EntryTrait;
use crate::bases::*;

pub struct EntryStore<Entry: EntryTrait> {
    idx: Delayed<EntryStoreIdx>,
    entries: Vec<Entry>,
    pub schema: schema::Schema,
}

impl<Entry: EntryTrait> EntryStore<Entry> {
    pub fn new(schema: schema::Schema) -> Self {
        Self {
            idx: Default::default(),
            entries: vec![],
            schema,
        }
    }

    pub fn add_entry(&mut self, entry: Entry) {
        self.schema.process(&entry);
        self.entries.push(entry);
    }

    pub fn get_idx(&self) -> EntryStoreIdx {
        self.idx.get()
    }
}

pub trait EntryStoreTrait: WritableTell {
    fn set_idx(&mut self, idx: EntryStoreIdx);
}

impl<Entry: EntryTrait> EntryStoreTrait for EntryStore<Entry> {
    fn set_idx(&mut self, idx: EntryStoreIdx) {
        self.idx.set(idx)
    }
}

impl<Entry: EntryTrait> WritableTell for EntryStore<Entry> {
    fn write_data(&self, _stream: &mut dyn OutStream) -> Result<()> {
        unreachable!();
    }

    fn write_tail(&self, _stream: &mut dyn OutStream) -> Result<()> {
        unreachable!();
    }

    fn write(&self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
        let layout = self.schema.finalize();
        for entry in &self.entries {
            layout.write_entry(entry, stream)?;
        }
        let offset = stream.tell();
        stream.write_u8(0x00)?; // kind
        layout.write(stream)?;
        stream.write_u64((self.entries.len() * layout.entry_size as usize) as u64)?;
        let size = stream.tell() - offset;
        Ok(SizedOffset { size, offset })
    }
}
