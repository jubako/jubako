use super::schema;
use super::FullEntryTrait;
use crate::bases::*;
use crate::creator::private::WritableTell;

fn set_entry_idx<Entry: FullEntryTrait>(entries: &mut Vec<Entry>) {
    let mut idx: EntryIdx = 0.into();
    for entry in entries {
        entry.set_idx(idx);
        idx += 1;
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

    pub fn add_entry(&mut self, mut entry: Entry) -> Bound<EntryIdx> {
        let entry_idx = entry.get_idx();
        entry.set_idx(EntryIdx::from(self.entries.len() as u32));
        self.entries.push(entry);
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
        set_entry_idx(&mut self.entries);
        if let Some(keys) = &self.schema.sort_keys {
            let compare = |a: &Entry, b: &Entry| a.compare(&mut keys.iter(), b);
            let compare_opt = |a: &Entry, b: &Entry| Some(a.compare(&mut keys.iter(), b));
            while !self.entries.is_sorted_by(compare_opt) {
                self.entries.sort_by(compare);
                set_entry_idx(&mut self.entries);
            }
        }

        for entry in &self.entries {
            self.schema.process(entry);
        }

        println!("Schema is {:#?}", self.schema);
    }
}

impl<Entry: FullEntryTrait> WritableTell for EntryStore<Entry> {
    fn write_data(&mut self, _stream: &mut dyn OutStream) -> Result<()> {
        unreachable!();
    }

    fn write_tail(&mut self, _stream: &mut dyn OutStream) -> Result<()> {
        unreachable!();
    }

    fn write(&mut self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
        let layout = self.schema.finalize();
        println!("Layout is {layout:#?}");
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
