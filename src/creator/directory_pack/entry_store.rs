use super::schema;
use super::{FullEntryTrait, PropertyName};
use crate::bases::*;
use crate::creator::private::WritableTell;

use log::debug;

fn set_entry_idx<PN: PropertyName, Entry: FullEntryTrait<PN>>(entries: &mut Vec<Entry>) {
    let mut idx: EntryIdx = 0.into();
    for entry in entries {
        entry.set_idx(idx);
        idx += 1;
    }
}

pub struct EntryStore<Entry: FullEntryTrait<PN>, PN: PropertyName> {
    idx: Late<EntryStoreIdx>,
    entries: Vec<Entry>,
    pub schema: schema::Schema<PN>,
}

impl<Entry: FullEntryTrait<PN>, PN: PropertyName> EntryStore<Entry, PN> {
    pub fn new(schema: schema::Schema<PN>) -> Self {
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

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

pub trait EntryStoreTrait: WritableTell {
    fn set_idx(&mut self, idx: EntryStoreIdx);
    fn finalize(&mut self);
}

impl<Entry: FullEntryTrait<PN>, PN: PropertyName + std::fmt::Debug> EntryStoreTrait
    for EntryStore<Entry, PN>
{
    fn set_idx(&mut self, idx: EntryStoreIdx) {
        self.idx.set(idx)
    }

    fn finalize(&mut self) {
        set_entry_idx(&mut self.entries);
        if let Some(keys) = &self.schema.sort_keys {
            let compare = |a: &Entry, b: &Entry| a.compare(&mut keys.iter(), b);
            let compare_opt = |a: &Entry, b: &Entry| Some(a.compare(&mut keys.iter(), b));
            let mut watchdog = 50;
            while !self.entries.is_sorted_by(compare_opt) {
                debug!(".");
                /*for entry in &self.entries {
                    println!("- {:?}/{:?}", entry.value(1.into()), entry.value(0.into()));
                }*/
                self.entries.sort_by(compare);
                set_entry_idx(&mut self.entries);
                watchdog -= 1;
                if watchdog == 0 {
                    panic!("Cannot sort entry store");
                }
            }
        }

        for entry in &self.entries {
            self.schema.process(entry);
        }

        debug!("Schema is {:#?}", self.schema);
    }
}

impl<Entry: FullEntryTrait<PN>, PN: PropertyName + std::fmt::Debug> WritableTell
    for EntryStore<Entry, PN>
{
    fn write_data(&mut self, _stream: &mut dyn OutStream) -> Result<()> {
        unreachable!();
    }

    fn write_tail(&mut self, _stream: &mut dyn OutStream) -> Result<()> {
        unreachable!();
    }

    fn write(&mut self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
        let layout = self.schema.finalize();
        debug!("Layout is {layout:#?}");
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
