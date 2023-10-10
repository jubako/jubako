use super::schema;
use super::{FullEntryTrait, PropertyName, VariantName};
use crate::bases::*;
use crate::creator::private::WritableTell;
use rayon::prelude::*;

use log::debug;

#[inline]
fn set_entry_idx<PN, VN, Entry>(entries: &mut [Entry])
where
    PN: PropertyName,
    VN: VariantName,
    Entry: FullEntryTrait<PN, VN> + Send + Sync,
{
    assert!(entries.len() <= u32::MAX as usize);
    //(0u32..).zip(entries.iter_mut()).for_each(|(idx, entry)| entry.set_idx(idx.into()))
    entries
        .par_iter_mut()
        .enumerate()
        .for_each(|(idx, entry)| entry.set_idx((idx as u32).into()))
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
    pub fn new(schema: schema::Schema<PN, VN>, size_hint: Option<usize>) -> Self {
        Self {
            entries: Vec::with_capacity(size_hint.unwrap_or(1024)),
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

pub trait EntryStoreTrait: Sync + Send {
    fn finalize(self: Box<Self>) -> Box<dyn WritableTell>;
}

impl<PN, VN, Entry> EntryStoreTrait for EntryStore<PN, VN, Entry>
where
    PN: PropertyName + std::fmt::Debug + Sync,
    VN: VariantName + std::fmt::Debug + Sync + 'static,
    Entry: FullEntryTrait<PN, VN> + Send + Sync + 'static,
{
    fn finalize(mut self: Box<Self>) -> Box<dyn WritableTell> {
        set_entry_idx(&mut self.entries);
        if let Some(keys) = &self.schema.sort_keys {
            let compare = |a: &Entry, b: &Entry| a.compare(&mut keys.iter(), b);
            self.entries.par_sort_unstable_by(compare);
            set_entry_idx(&mut self.entries);
            let mut watchdog = 50;
            while !self
                .entries
                .par_windows(2)
                .all(|w| w[0].compare(&mut keys.iter(), &w[1]).is_le())
            {
                debug!(".");
                self.entries.par_sort_unstable_by(compare);
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
