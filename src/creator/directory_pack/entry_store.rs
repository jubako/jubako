use super::schema;
use super::{FullEntryTrait, PropertyName, VariantName};
use crate::bases::*;
use crate::creator::private::WritableTell;
use crate::creator::Result;

use log::debug;

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
    pub fn new(schema: schema::Schema<PN, VN>, entries: Vec<Entry>) -> Self {
        Self { entries, schema }
    }
}

pub trait EntryStoreTrait {
    fn finalize(self: Box<Self>) -> Box<dyn WritableTell>;
}

impl<PN, VN, Entry> EntryStoreTrait for EntryStore<PN, VN, Entry>
where
    PN: PropertyName + std::fmt::Debug + Sync,
    VN: VariantName + std::fmt::Debug + Sync + 'static,
    Entry: FullEntryTrait<PN, VN> + Send + 'static,
{
    fn finalize(mut self: Box<Self>) -> Box<dyn WritableTell> {
        if let Some(keys) = &self.schema.sort_keys {
            if !self
                .entries
                .windows(2)
                .all(|w| w[0].compare(&keys, &w[1]).is_le())
            {
                panic!("Entry store is not sorted");
            }
        }
        for entry in &mut self.entries {
            self.schema.process(entry);
        }

        debug!("Schema is {:#?}", self.schema);

        let layout = self.schema.finalize();
        Box::new(FinalEntryStore {
            entry_count: self.entries.len() as u32,
            entries: self.entries.into_iter(),
            layout,
        })
    }
}

struct FinalEntryStore<PN, VN, Entry, Store>
where
    PN: PropertyName,
    VN: VariantName,
    Entry: FullEntryTrait<PN, VN>,
    Store: Iterator<Item = Entry>,
{
    entry_count: u32,
    entries: Store,
    layout: super::layout::Entry<PN, VN>,
}

impl<PN, VN, Entry, Store> WritableTell for FinalEntryStore<PN, VN, Entry, Store>
where
    PN: PropertyName + std::fmt::Debug,
    VN: VariantName + std::fmt::Debug,
    Entry: FullEntryTrait<PN, VN>,
    Store: Iterator<Item = Entry>,
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
