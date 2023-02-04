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

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(Word<u64>),
    Signed(i64),
    Array { data: Vec<u8>, value_id: Bound<u64> },
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<cmp::Ordering> {
        match self {
            Value::Content(_) => None,
            Value::Unsigned(v) => match other {
                Value::Unsigned(o) => Some(v.get().cmp(&o.get())),
                _ => None,
            },
            Value::Signed(v) => match other {
                Value::Signed(o) => Some(v.cmp(o)),
                _ => None,
            },
            Value::Array { data, value_id: id } => match other {
                Value::Array {
                    data: other_data,
                    value_id: other_id,
                } => match data.cmp(other_data) {
                    cmp::Ordering::Less => Some(cmp::Ordering::Less),
                    cmp::Ordering::Greater => Some(cmp::Ordering::Greater),
                    cmp::Ordering::Equal => Some(id.get().cmp(&other_id.get())),
                },
                _ => None,
            },
        }
    }
}

pub trait EntryTrait {
    fn variant_id(&self) -> Option<VariantIdx>;
    fn value(&self, id: PropertyIdx) -> &Value;
    fn value_count(&self) -> PropertyCount;
    fn set_idx(&mut self, idx: EntryIdx);
    fn get_idx(&self) -> Bound<EntryIdx>;
}

pub trait FullEntryTrait: EntryTrait {
    fn compare(&self, sort_keys: &mut dyn Iterator<Item = &PropertyIdx>, other: &Self) -> bool;
}

struct EntryIter<'e> {
    entry: &'e dyn EntryTrait,
    idx: PropertyIdx,
}

impl<'e> EntryIter<'e> {
    fn new(entry: &'e dyn EntryTrait) -> Self {
        Self {
            entry,
            idx: PropertyIdx::from(0),
        }
    }
}

impl<'e> Iterator for EntryIter<'e> {
    type Item = &'e Value;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx.is_valid(self.entry.value_count()) {
            let value = self.entry.value(self.idx);
            self.idx += 1;
            Some(value)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct BasicEntry {
    variant_id: Option<VariantIdx>,
    values: Vec<Value>,
    idx: Vow<EntryIdx>,
}

pub struct ValueTransformer<'a> {
    keys: Box<dyn Iterator<Item = &'a schema::Property> + 'a>,
    values: std::vec::IntoIter<common::Value>,
}

impl<'a> ValueTransformer<'a> {
    pub fn new(
        schema: &'a schema::Schema,
        variant_id: Option<VariantIdx>,
        values: Vec<common::Value>,
    ) -> Self {
        if schema.variants.is_empty() {
            ValueTransformer {
                keys: Box::new(schema.common.iter()),
                values: values.into_iter(),
            }
        } else {
            let keys = schema
                .common
                .iter()
                .chain(schema.variants[variant_id.unwrap().into_usize()].iter());
            ValueTransformer {
                keys: Box::new(keys),
                values: values.into_iter(),
            }
        }
    }
}

impl<'a> Iterator for ValueTransformer<'a> {
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
    pub fn new_from_schema(
        schema: &schema::Schema,
        variant_id: Option<VariantIdx>,
        values: Vec<common::Value>,
    ) -> Self {
        let value_transformer = ValueTransformer::new(schema, variant_id, values);
        Self::new(variant_id, value_transformer.collect())
    }

    pub fn new(variant_id: Option<VariantIdx>, values: Vec<Value>) -> Self {
        Self {
            variant_id,
            values,
            idx: Default::default(),
        }
    }
}

impl EntryTrait for BasicEntry {
    fn variant_id(&self) -> Option<VariantIdx> {
        self.variant_id
    }
    fn value(&self, id: PropertyIdx) -> &Value {
        &self.values[id.into_usize()]
    }
    fn value_count(&self) -> PropertyCount {
        (self.values.len() as u8).into()
    }
    fn set_idx(&mut self, idx: EntryIdx) {
        self.idx.fulfil(idx);
    }
    fn get_idx(&self) -> Bound<EntryIdx> {
        self.idx.bind()
    }
}

impl<T> EntryTrait for Box<T>
where
    T: EntryTrait,
{
    fn variant_id(&self) -> Option<VariantIdx> {
        T::variant_id(self)
    }
    fn value(&self, id: PropertyIdx) -> &Value {
        T::value(self, id)
    }
    fn value_count(&self) -> PropertyCount {
        T::value_count(self)
    }
    fn set_idx(&mut self, idx: EntryIdx) {
        T::set_idx(self, idx)
    }
    fn get_idx(&self) -> Bound<EntryIdx> {
        T::get_idx(self)
    }
}

impl FullEntryTrait for BasicEntry {
    fn compare(
        &self,
        sort_keys: &mut dyn Iterator<Item = &PropertyIdx>,
        other: &BasicEntry,
    ) -> bool {
        for &property_id in sort_keys {
            let self_value = self.value(property_id);
            let other_value = other.value(property_id);
            match self_value.partial_cmp(other_value) {
                None => return false,
                Some(c) => match c {
                    cmp::Ordering::Less => return true,
                    cmp::Ordering::Greater => return false,
                    cmp::Ordering::Equal => continue,
                },
            }
        }
        false
    }
}

impl<T> FullEntryTrait for Box<T>
where
    T: FullEntryTrait,
{
    fn compare(&self, sort_keys: &mut dyn Iterator<Item = &PropertyIdx>, other: &Self) -> bool {
        T::compare(self, sort_keys, other)
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

impl super::private::WritableTell for Index {
    fn write_data(&mut self, _stream: &mut dyn OutStream) -> Result<()> {
        // No data to write
        Ok(())
    }
    fn write_tail(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        self.store_id.write(stream)?;
        self.count.write(stream)?;
        self.offset.write(stream)?;
        self.extra_data.write(stream)?;
        self.index_key.write(stream)?;
        PString::write_string(self.name.as_ref(), stream)?;
        Ok(())
    }
}
