use super::schema;
use super::{FullEntryTrait, PropertyName, VariantName};
use crate::bases::*;
use crate::creator::private::WritableTell;

use log::debug;

fn set_entry_idx<PN, VN, Entry>(entries: &mut Vec<Entry>)
where
    PN: PropertyName,
    VN: VariantName,
    Entry: FullEntryTrait<PN, VN>,
{
    let mut idx: EntryIdx = 0.into();
    for entry in entries {
        entry.set_idx(idx);
        idx += 1;
    }
}

pub struct EntryStore<PN, VN, Entry>
where
    PN: PropertyName,
    VN: VariantName,
    Entry: FullEntryTrait<PN, VN>,
{
    entries: Vec<Entry>,
    pub schema: schema::Schema<PN, VN>,
}

impl<PN, VN, Entry> EntryStore<PN, VN, Entry>
where
    PN: PropertyName,
    VN: VariantName,
    Entry: FullEntryTrait<PN, VN>,
{
    pub fn new(schema: schema::Schema<PN, VN>) -> Self {
        Self {
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

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }
}

pub trait EntryStoreTrait {
    fn finalize(self: Box<Self>) -> Box<dyn WritableTell>;
}

impl<PN, VN, Entry> EntryStoreTrait for EntryStore<PN, VN, Entry>
where
    PN: PropertyName + std::fmt::Debug,
    VN: VariantName + std::fmt::Debug + 'static,
    Entry: FullEntryTrait<PN, VN> + 'static,
{
    fn finalize(mut self: Box<Self>) -> Box<dyn WritableTell> {
        set_entry_idx(&mut self.entries);
        if let Some(keys) = &self.schema.sort_keys {
            let compare = |a: &Entry, b: &Entry| a.compare(&mut keys.iter(), b);
            let compare_opt = |a: &Entry, b: &Entry| Some(a.compare(&mut keys.iter(), b));
            let mut watchdog = 50;
            while !self.entries.is_sorted_by(compare_opt) {
                debug!(".");
                self.entries.sort_by(compare);
                set_entry_idx(&mut self.entries);
                watchdog -= 1;
                if watchdog == 0 {
                    panic!("Cannot sort entry store");
                }
            }
        }

        for entry in &mut self.entries {
            self.schema.process(entry);
        }

        debug!("Schema is {:#?}", self.schema);

        let layout = self.schema.finalize();
        Box::new(FinalEntryStore {
            entries: self.entries,
            layout,
        })
    }
}

pub struct FinalEntryStore<PN, VN, Entry>
where
    PN: PropertyName,
    VN: VariantName,
    Entry: FullEntryTrait<PN, VN>,
{
    entries: Vec<Entry>,
    layout: super::layout::Entry<PN, VN>,
}

impl<PN, VN, Entry> WritableTell for FinalEntryStore<PN, VN, Entry>
where
    PN: PropertyName + std::fmt::Debug,
    VN: VariantName + std::fmt::Debug,
    Entry: FullEntryTrait<PN, VN>,
{
    fn write_data(&mut self, _stream: &mut dyn OutStream) -> Result<()> {
        unreachable!();
    }

    fn write_tail(&mut self, _stream: &mut dyn OutStream) -> Result<()> {
        unreachable!();
    }

    fn write(&mut self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
        debug!("Layout is {:#?}", self.layout);
        for entry in &self.entries {
            self.layout.write_entry(entry, stream)?;
        }
        let offset = stream.tell();
        stream.write_u8(0x00)?; // kind
        self.layout.write(stream)?;
        stream.write_u64((self.entries.len() * self.layout.entry_size as usize) as u64)?;
        let size = stream.tell() - offset;
        Ok(SizedOffset { size, offset })
    }
}
