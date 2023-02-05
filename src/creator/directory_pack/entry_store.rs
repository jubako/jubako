use super::private::WritableTell;
use super::schema;
use super::FullEntryTrait;
use crate::bases::*;

struct EntryCompare<'e, Entry: FullEntryTrait> {
    pub ref_entry: &'e Entry,
    pub property_ids: &'e Vec<PropertyIdx>,
}

impl<'e, Entry: FullEntryTrait> EntryCompare<'e, Entry> {
    fn compare(&self, other_entry: &Entry) -> bool {
        other_entry.compare(&mut self.property_ids.iter(), self.ref_entry)
    }
}

pub struct EntryStore<Entry: FullEntryTrait> {
    idx: Late<EntryStoreIdx>,
    entries: Vec<Entry>,
    pub schema: schema::Schema,
}

impl<Entry: FullEntryTrait> EntryStore<Entry> {
    pub fn new(schema: schema::Schema) -> Self {
        Self {
            idx: Default::default(),
            entries: vec![],
            schema,
        }
    }

    pub fn add_entry(&mut self, entry: Entry) -> Bound<EntryIdx> {
        let entry_idx = entry.get_idx();
        match &self.schema.sort_keys {
            None => self.entries.push(entry),
            Some(keys) => {
                let comparator = EntryCompare {
                    ref_entry: &entry,
                    property_ids: keys,
                };
                let idx = self.entries.partition_point(|e| comparator.compare(e));
                self.entries.insert(idx, entry);
            }
        };
        entry_idx
    }

    pub fn get_idx(&self) -> EntryStoreIdx {
        self.idx.get()
    }
}

pub trait EntryStoreTrait: WritableTell {
    fn set_idx(&mut self, idx: EntryStoreIdx);
    fn finalize(&mut self);
}

impl<Entry: FullEntryTrait> EntryStoreTrait for EntryStore<Entry> {
    fn set_idx(&mut self, idx: EntryStoreIdx) {
        self.idx.set(idx)
    }

    fn finalize(&mut self) {
        let mut idx: EntryIdx = 0.into();
        for entry in &mut self.entries {
            entry.set_idx(idx);
            idx += 1;
        }

        for entry in &self.entries {
            self.schema.process(entry);
        }

        println!("Schema is {:#?}", self.schema);
    }
}

impl<Entry: FullEntryTrait> WritableTell for EntryStore<Entry> {
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
