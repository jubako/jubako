#[allow(clippy::module_inception)]
mod directory_pack;
mod entry_store;
mod layout;
pub mod schema;
mod simple_entry;
mod value;
mod value_store;

use crate::bases::*;
use crate::common;
use crate::creator::Result;
pub use directory_pack::DirectoryPackCreator;
pub use entry_store::EntryStore;
pub use simple_entry::SimpleEntry;
use std::cmp::{self, PartialOrd};
use std::marker::PhantomData;
pub use value::{Array, ArrayS, ProcessedValue};
pub(crate) use value_store::ValueStoreKind;
pub use value_store::{StoreHandle, ValueHandle, ValueStore};

pub trait EntryTrait<PN: PropertyName, VN: VariantName> {
    fn variant_name(&self) -> Option<VN>;
    fn value(&self, name: &PN) -> common::Value;
    fn value_count(&self) -> PropertyCount;
    fn compare<'i, I>(&self, sort_keys: &'i I, other: &Self) -> std::cmp::Ordering
    where
        I: IntoIterator<Item = &'i PN> + Copy,
    {
        for property_name in sort_keys.into_iter() {
            let self_value = self.value(property_name);
            let other_value = other.value(property_name);
            match self_value.partial_cmp(&other_value) {
                None => return cmp::Ordering::Greater,
                Some(c) => match c {
                    cmp::Ordering::Less => return cmp::Ordering::Less,
                    cmp::Ordering::Greater => return cmp::Ordering::Greater,
                    cmp::Ordering::Equal => continue,
                },
            }
        }
        cmp::Ordering::Greater
    }
}

#[derive(Debug)]
pub struct ProcessedEntry<VN> {
    variant_name: Option<VN>,
    values: Box<[ProcessedValue]>,
}

/// ValueTransformer is responsible to transform `common::Value` (used outside of Jubako)
/// into `creator::Value`, a value we can write in container, according to a schema.
/// For example, it replace a array value (&[u8]) to a
/// creator::Value::Array(base_array + idx to value stored in value_store)
pub(crate) struct ValueTransformer<'a, PN: PropertyName, VN: VariantName, Entry: EntryTrait<PN, VN>>
{
    keys: Box<dyn Iterator<Item = &'a mut schema::Property<PN>> + 'a>,
    entry: Entry,
    _phantom: PhantomData<VN>,
}

impl<'a, PN: PropertyName, VN: VariantName, Entry: EntryTrait<PN, VN>>
    ValueTransformer<'a, PN, VN, Entry>
{
    /// Create a new ValueTransformer
    /// `variant_name` and `values` must match the schema:
    /// - If schema contain a variant, variant_name must be Some(...)
    /// - values hashmap must contains values corresponding to the properties
    ///   declared in the schema (variant).
    pub fn new(schema: &'a mut schema::Schema<PN, VN>, entry: Entry) -> Self {
        let variant_name = entry.variant_name();
        let keys: Option<Box<dyn Iterator<Item = _>>> = if schema.variants.is_empty() {
            Some(Box::new(schema.common.iter_mut()))
        } else {
            let common = &mut schema.common;
            let variant = schema
                .variants
                .iter_mut()
                .find(|(n, _)| n == &variant_name.unwrap());
            if let Some((_, v)) = variant {
                let keys =
                    Box::new(common.iter_mut().chain(v.iter_mut())) as Box<dyn Iterator<Item = _>>;
                Some(keys)
            } else {
                None
            }
        };

        if let Some(keys) = keys {
            ValueTransformer {
                keys,
                entry,
                _phantom: Default::default(),
            }
        } else {
            //[TODO] Transform this as Result
            panic!(
                "Entry variant name {} doesn't correspond to possible variants",
                variant_name.unwrap().as_str()
            );
        }
    }
}

impl<PN: PropertyName, VN: VariantName, Entry: EntryTrait<PN, VN>> Iterator
    for ValueTransformer<'_, PN, VN, Entry>
{
    type Item = ProcessedValue;
    // Iter on all `common::Value` and produce `(PN, creator::Value)`
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.keys.next() {
                None => return None,
                Some(ref mut key) => match key {
                    schema::Property::Array(prop, name) => {
                        let value = self.entry.value(name);
                        if let common::Value::Array(data) = value {
                            return Some(prop.absorb(data));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::IndirectArray(prop, name) => {
                        let value = self.entry.value(name);
                        if let common::Value::Array(data) = value {
                            return Some(prop.absorb(data));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::UnsignedInt(prop, name) => {
                        let value = self.entry.value(name);
                        if let common::Value::Unsigned(v) = value {
                            return Some(prop.absorb(v));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::SignedInt(prop, name) => {
                        let value = self.entry.value(name);
                        if let common::Value::Signed(v) = value {
                            return Some(prop.absorb(v));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::ContentAddress(prop, name) => {
                        let value = self.entry.value(name);
                        if let common::Value::Content(v) = value {
                            return Some(prop.absorb(v));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::Padding(_) => {}
                },
            }
        }
    }
}

struct Index {
    store_id: EntryStoreIdx,
    free_data: IndexFreeData,
    index_key: PropertyIdx,
    name: String,
    count: EntryCount,
    offset: EntryIdx,
}

impl Index {
    pub fn new(
        name: &str,
        free_data: IndexFreeData,
        index_key: PropertyIdx,
        store_id: EntryStoreIdx,
        count: EntryCount,
        offset: EntryIdx,
    ) -> Self {
        Index {
            store_id,
            free_data,
            index_key,
            name: name.to_string(),
            count,
            offset,
        }
    }
}

impl super::private::WritableTell for Index {
    fn write_data(&mut self, _stream: &mut dyn OutStream) -> Result<()> {
        // No data to write
        Ok(())
    }
    fn serialize_tail(&mut self, ser: &mut Serializer) -> IoResult<()> {
        self.store_id.serialize(ser)?;
        self.count.serialize(ser)?;
        self.offset.serialize(ser)?;
        self.free_data.serialize(ser)?;
        self.index_key.serialize(ser)?;
        PString::serialize_string(&self.name, ser)?;
        Ok(())
    }
}
