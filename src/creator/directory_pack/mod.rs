#[allow(clippy::module_inception)]
mod directory_pack;
mod entry_store;
mod layout;
pub mod schema;
mod value;
mod value_store;

use crate::bases::*;
use crate::common;
use crate::creator::Result;
pub use directory_pack::DirectoryPackCreator;
pub use entry_store::EntryStore;
use std::cmp;
use std::collections::HashMap;
pub use value::{Array, ArrayS, Value};
pub(crate) use value_store::ValueStoreKind;
pub use value_store::{StoreHandle, ValueHandle, ValueStore};

pub trait EntryTrait<PN: PropertyName, VN: VariantName> {
    fn variant_name(&self) -> Option<MayRef<'_, VN>>;
    fn value<'a>(&'a self, name: &PN) -> MayRef<'a, Value>;
    fn value_count(&self) -> PropertyCount;
}

pub trait FullEntryTrait<PN: PropertyName, VN: VariantName>: EntryTrait<PN, VN> + Send {
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
pub struct BasicEntry<VN> {
    variant_name: Option<VN>,
    values: Box<[Value]>,
}

/// ValueTransformer is responsible to transform `common::Value` (used outside of Jubako)
/// into `creator::Value`, a value we can write in container, according to a schema.
/// For example, it replace a array value (&[u8]) to a
/// creator::Value::Array(base_array + idx to value stored in value_store)
pub(crate) struct ValueTransformer<'a, PN: PropertyName> {
    keys: Box<dyn Iterator<Item = &'a mut schema::Property<PN>> + 'a>,
    values: HashMap<PN, common::Value>,
}

impl<'a, PN: PropertyName> ValueTransformer<'a, PN> {
    /// Create a new ValueTransformer
    /// `variant_name` and `values` must match the schema:
    /// - If schema contain a variant, variant_name must be Some(...)
    /// - values hashmap must contains values corresponding to the properties
    ///   declared in the schema (variant).
    pub fn new<VN: VariantName>(
        schema: &'a mut schema::Schema<PN, VN>,
        variant_name: &Option<VN>,
        values: HashMap<PN, common::Value>,
    ) -> Self {
        let keys: Option<Box<dyn Iterator<Item = _>>> = if schema.variants.is_empty() {
            Some(Box::new(schema.common.iter_mut()))
        } else {
            let variant_name = variant_name.as_ref().unwrap();
            let common = &mut schema.common;
            let variant = schema.variants.iter_mut().find(|(n, _)| n == variant_name);
            if let Some((_, v)) = variant {
                let keys =
                    Box::new(common.iter_mut().chain(v.iter_mut())) as Box<dyn Iterator<Item = _>>;
                Some(keys)
            } else {
                None
            }
        };

        if let Some(keys) = keys {
            ValueTransformer { keys, values }
        } else {
            //[TODO] Transform this as Result
            panic!(
                "Entry variant name {} doesn't correspond to possible variants",
                variant_name.as_ref().unwrap().as_str()
            );
        }
    }
}

impl<PN: PropertyName> Iterator for ValueTransformer<'_, PN> {
    type Item = Value;
    // Iter on all `common::Value` and produce `(PN, creator::Value)`
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.keys.next() {
                None => return None,
                Some(ref mut key) => match key {
                    schema::Property::Array(prop, name) => {
                        let value = self
                            .values
                            .remove(name)
                            .unwrap_or_else(|| panic!("Cannot find entry {}", name.as_str()));
                        if let common::Value::Array(data) = value {
                            return Some(prop.absorb(data));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::IndirectArray(prop, name) => {
                        let value = self
                            .values
                            .remove(name)
                            .unwrap_or_else(|| panic!("Cannot find entry {}", name.as_str()));
                        if let common::Value::Array(data) = value {
                            return Some(prop.absorb(data));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::UnsignedInt(prop, name) => {
                        let value = self
                            .values
                            .remove(name)
                            .unwrap_or_else(|| panic!("Cannot find entry {}", name.as_str()));

                        if let common::Value::Unsigned(v) = value {
                            return Some(prop.absorb(v));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::SignedInt(prop, name) => {
                        let value = self
                            .values
                            .remove(name)
                            .unwrap_or_else(|| panic!("Cannot find entry {}", name.as_str()));

                        if let common::Value::Signed(v) = value {
                            return Some(prop.absorb(v));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::ContentAddress(prop, name) => {
                        let value = self
                            .values
                            .remove(name)
                            .unwrap_or_else(|| panic!("Cannot find entry {}", name.as_str()));
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

impl<VN: VariantName> BasicEntry<VN> {
    pub fn new_from_schema<PN: PropertyName>(
        schema: &mut schema::Schema<PN, VN>,
        variant_name: Option<VN>,
        values: HashMap<PN, common::Value>,
    ) -> Self {
        let value_transformer = ValueTransformer::<PN>::new(schema, &variant_name, values);
        Self::new(variant_name, value_transformer.collect())
    }

    pub(crate) fn new(variant_name: Option<VN>, values: Vec<Value>) -> Self {
        Self {
            variant_name,
            values: values.into(),
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
