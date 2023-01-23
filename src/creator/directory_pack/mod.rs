#[allow(clippy::module_inception)]
mod directory_pack;
mod entry_store;
mod layout;
pub mod schema;
mod value_store;

use crate::bases::*;
use crate::common;
use crate::common::ContentAddress;
pub use directory_pack::DirectoryPackCreator;
pub use entry_store::EntryStore;
use std::cmp;
use value_store::ValueStore;
pub use value_store::ValueStoreKind;

mod private {
    use super::*;
    pub trait WritableTell {
        fn write_data(&self, stream: &mut dyn OutStream) -> Result<()>;
        fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()>;
        fn write(&self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
            self.write_data(stream)?;
            let offset = stream.tell();
            self.write_tail(stream)?;
            let size = stream.tell() - offset;
            Ok(SizedOffset { size, offset })
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(u64),
    Signed(i64),
    Array { data: Vec<u8>, value_id: u64 },
}

pub trait EntryTrait {
    fn variant_id(&self) -> Option<VariantIdx>;
    fn values(&self) -> Vec<Value>;
}

#[derive(Debug)]
pub struct BasicEntry {
    variant_id: Option<VariantIdx>,
    values: Vec<Value>,
}

struct ValueTransformer<'a, T1, T2>
where
    T1: Iterator<Item = &'a schema::Property>,
    T2: Iterator<Item = common::Value>,
{
    keys: T1,
    values: T2,
}

impl<'a, T1, T2> Iterator for ValueTransformer<'a, T1, T2>
where
    T1: Iterator<Item = &'a schema::Property>,
    T2: Iterator<Item = common::Value>,
{
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.keys.next() {
                None => return None,
                Some(key) => match key {
                    schema::Property::VLArray(flookup_size, store_handle) => {
                        let flookup_size = flookup_size;
                        let value = self.values.next().unwrap();
                        if let common::Value::Array(mut data) = value {
                            let to_store = data.split_off(cmp::min(*flookup_size, data.len()));
                            let value_id = store_handle.borrow_mut().add_value(&to_store);
                            return Some(Value::Array { data, value_id });
                        } else {
                            panic!("Invalide value type");
                        }
                    }
                    schema::Property::UnsignedInt(_) => {
                        let value = self.values.next().unwrap();
                        if let common::Value::Unsigned(v) = value {
                            return Some(Value::Unsigned(v));
                        } else {
                            panic!("Invalide value type");
                        }
                    }
                    schema::Property::ContentAddress => {
                        let value = self.values.next().unwrap();
                        if let common::Value::Content(v) = value {
                            return Some(Value::Content(v));
                        } else {
                            panic!("Invalide value type");
                        }
                    }
                    schema::Property::Padding(_) => {}
                },
            }
        }
    }
}

impl BasicEntry {
    pub fn new(
        schema: &schema::Schema,
        variant_id: Option<VariantIdx>,
        values: Vec<common::Value>,
    ) -> Self {
        let values: Vec<Value> = if schema.variants.is_empty() {
            ValueTransformer {
                keys: schema.common.iter(),
                values: values.into_iter(),
            }
            .collect()
        } else {
            let keys = schema
                .common
                .iter()
                .chain(schema.variants[variant_id.unwrap().into_usize()].iter());
            ValueTransformer {
                keys,
                values: values.into_iter(),
            }
            .collect()
        };
        Self { variant_id, values }
    }
}

impl EntryTrait for BasicEntry {
    fn variant_id(&self) -> Option<VariantIdx> {
        self.variant_id
    }
    fn values(&self) -> Vec<Value> {
        self.values.clone()
    }
}

struct Index {
    store_id: EntryStoreIdx,
    extra_data: ContentAddress,
    index_key: PropertyIdx,
    name: String,
    count: EntryCount,
    offset: EntryIdx,
}

impl Index {
    pub fn new(
        name: &str,
        extra_data: ContentAddress,
        index_key: PropertyIdx,
        store_id: EntryStoreIdx,
        count: EntryCount,
        offset: EntryIdx,
    ) -> Self {
        Index {
            store_id,
            extra_data,
            index_key,
            name: name.to_string(),
            count,
            offset,
        }
    }
}

impl private::WritableTell for Index {
    fn write_data(&self, _stream: &mut dyn OutStream) -> Result<()> {
        // No data to write
        Ok(())
    }
    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        self.store_id.write(stream)?;
        self.count.write(stream)?;
        self.offset.write(stream)?;
        self.extra_data.write(stream)?;
        self.index_key.write(stream)?;
        PString::write_string(self.name.as_ref(), stream)?;
        Ok(())
    }
}
