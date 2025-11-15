use super::schema;
use super::{PropertyName, VariantName};
use crate::bases::*;
use crate::creator::private::WritableTell;
use crate::creator::{BasicEntry, EntryTrait, Result};

use log::debug;

pub struct EntryStore<PN, VN>
where
    PN: PropertyName,
    VN: VariantName,
{
    entries: Vec<BasicEntry<VN>>,
    pub schema: schema::Schema<PN, VN>,
}

impl<PN, VN> EntryStore<PN, VN>
where
    PN: PropertyName,
    VN: VariantName,
{
    pub fn new<Entry: EntryTrait<PN, VN>>(
        mut schema: schema::Schema<PN, VN>,
        entries: Vec<Entry>,
    ) -> Self {
        let entries = schema.process_entries(entries);
        Self { entries, schema }
    }
}

pub trait EntryStoreTrait {
    fn finalize(self: Box<Self>) -> Box<dyn WritableTell>;
}

impl<PN, VN> EntryStoreTrait for EntryStore<PN, VN>
where
    PN: PropertyName + std::fmt::Debug + Sync,
    VN: VariantName + std::fmt::Debug + Sync + 'static,
{
    fn finalize(self: Box<Self>) -> Box<dyn WritableTell> {
        /* [TODO] Move into schema process_entries
                if let Some(keys) = &self.schema.sort_keys {
                    if !self
                        .entries
                        .windows(2)
                        .all(|w| w[0].compare(&keys, &w[1]).is_le())
                    {
                        panic!("Entry store is not sorted");
                    }
                }
        */
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
    Store: Iterator<Item = BasicEntry<VN>>,
{
    entry_count: u32,
    entries: Store,
    layout: super::layout::Entry<PN, VN>,
}

impl<PN, VN, Store> WritableTell for FinalEntryStore<PN, VN, Store>
where
    PN: PropertyName + std::fmt::Debug,
    VN: VariantName + std::fmt::Debug,
    Store: Iterator<Item = BasicEntry<VN>>,
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
