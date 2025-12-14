use super::schema;
use super::{PropertyName, VariantName};
use crate::bases::*;
use crate::creator::private::WritableTell;
use crate::creator::{EntryTrait, ProcessedEntry, Result};

use log::debug;

pub struct EntryStore<PN: PropertyName, VN: VariantName> {
    entries: Vec<ProcessedEntry<VN>>,
    pub schema: schema::Schema<PN, VN>,
}

impl<PN: PropertyName, VN: VariantName> EntryStore<PN, VN> {
    pub fn new<Entry: EntryTrait<PN, VN>>(
        mut schema: schema::Schema<PN, VN>,
        entries: impl Iterator<Item = Entry>,
    ) -> Self {
        let entries = schema.process_entries(entries);
        Self { entries, schema }
    }
}

pub(crate) trait EntryStoreTrait {
    fn finalize(self: Box<Self>) -> Box<dyn WritableTell>;
}

impl<PN: PropertyName, VN: VariantName> EntryStoreTrait for EntryStore<PN, VN> {
    fn finalize(self: Box<Self>) -> Box<dyn WritableTell> {
        debug!("Schema is {:#?}", self.schema);

        let layout = self.schema.finalize();
        Box::new(FinalEntryStore {
            entry_count: self.entries.len() as u32,
            entries: self.entries.into_iter(),
            layout,
        })
    }
}

struct FinalEntryStore<PN, VN, Store>
where
    PN: PropertyName,
    VN: VariantName,
    Store: Iterator<Item = ProcessedEntry<VN>>,
{
    entry_count: u32,
    entries: Store,
    layout: super::layout::Entry<PN, VN>,
}

impl<PN, VN, Store> WritableTell for FinalEntryStore<PN, VN, Store>
where
    PN: PropertyName,
    VN: VariantName,
    Store: Iterator<Item = ProcessedEntry<VN>>,
{
    fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        // [TODO] Handle per entry CRC32
        let mut serializer = Serializer::new(BlockCheck::Crc32);
        for entry in &mut self.entries {
            self.layout.serialize_entry(&entry, &mut serializer)?;
        }
        stream.write_serializer(serializer)?;
        Ok(())
    }

    fn serialize_tail(&mut self, ser: &mut Serializer) -> std::io::Result<()> {
        ser.write_u8(0x00)?; // kind
        let entry_count = EntryCount::from(self.entry_count);
        entry_count.serialize(ser)?;
        // [TODO] handle per entry CRC32
        ser.write_u8(0x00)?; // flag
        self.layout.serialize(ser)?;
        Ok(())
    }
}
