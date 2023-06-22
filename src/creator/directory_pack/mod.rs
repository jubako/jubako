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
use std::collections::HashMap;
use value_store::ValueStore;
pub use value_store::ValueStoreKind;

#[derive(Debug, PartialEq)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(Word<u64>),
    Signed(Word<i64>),
    Array {
        size: usize,
        data: Vec<u8>,
        value_id: Bound<u64>,
    },
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
                Value::Signed(o) => Some(v.get().cmp(&o.get())),
                _ => None,
            },
            Value::Array {
                size,
                data,
                value_id: id,
            } => match other {
                Value::Array {
                    size: other_size,
                    data: other_data,
                    value_id: other_id,
                } => match data.cmp(other_data) {
                    cmp::Ordering::Less => Some(cmp::Ordering::Less),
                    cmp::Ordering::Greater => Some(cmp::Ordering::Greater),
                    cmp::Ordering::Equal => match id.get().cmp(&other_id.get()) {
                        cmp::Ordering::Less => Some(cmp::Ordering::Less),
                        cmp::Ordering::Greater => Some(cmp::Ordering::Greater),
                        cmp::Ordering::Equal => Some(size.cmp(other_size)),
                    },
                },
                _ => None,
            },
        }
    }
}

pub trait EntryTrait {
    fn variant_name(&self) -> Option<&str>;
    fn value(&self, name: &str) -> &Value;
    fn value_count(&self) -> PropertyCount;
    fn set_idx(&mut self, idx: EntryIdx);
    fn get_idx(&self) -> Bound<EntryIdx>;
}

pub trait FullEntryTrait: EntryTrait {
    fn compare(
        &self,
        sort_keys: &mut dyn Iterator<Item = &String>,
        other: &Self,
    ) -> std::cmp::Ordering;
}

#[derive(Debug)]
pub struct BasicEntry {
    variant_name: Option<String>,
    values: HashMap<String, Value>,
    idx: Vow<EntryIdx>,
}

pub struct ValueTransformer<'a> {
    keys: Box<dyn Iterator<Item = &'a schema::Property> + 'a>,
    values: HashMap<String, common::Value>,
}

impl<'a> ValueTransformer<'a> {
    pub fn new(
        schema: &'a schema::Schema,
        variant_name: &Option<String>,
        values: HashMap<String, common::Value>,
    ) -> Self {
        if schema.variants.is_empty() {
            return ValueTransformer {
                keys: Box::new(schema.common.iter()),
                values,
            };
        } else {
            for (n, v) in &schema.variants {
                if n == variant_name.as_ref().unwrap() {
                    let keys = schema.common.iter().chain(v.iter());
                    return ValueTransformer {
                        keys: Box::new(keys),
                        values,
                    };
                }
            }
            //[TODO] Transform this as Result
            panic!(
                "Entry variant name {} doesn't correspond to possible variants",
                variant_name.as_ref().unwrap()
            );
        };
    }
}

impl<'a> Iterator for ValueTransformer<'a> {
    type Item = (String, Value);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.keys.next() {
                None => return None,
                Some(key) => match key {
                    schema::Property::Array {
                        max_array_size: _,
                        fixed_array_size,
                        store_handle,
                        name,
                    } => {
                        let value = self.values.remove(name).unwrap();
                        if let common::Value::Array(mut data) = value {
                            let size = data.len();
                            let to_store = data.split_off(cmp::min(*fixed_array_size, data.len()));
                            let value_id = store_handle.borrow_mut().add_value(&to_store);
                            return Some((
                                name.to_string(),
                                Value::Array {
                                    size,
                                    data,
                                    value_id,
                                },
                            ));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::UnsignedInt {
                        counter: _,
                        size: _,
                        name,
                    } => {
                        let value = self.values.remove(name).unwrap();
                        if let common::Value::Unsigned(v) = value {
                            return Some((name.to_string(), Value::Unsigned(v)));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::SignedInt {
                        counter: _,
                        size: _,
                        name,
                    } => {
                        let value = self.values.remove(name).unwrap();
                        if let common::Value::Signed(v) = value {
                            return Some((name.to_string(), Value::Signed(v)));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::ContentAddress {
                        pack_id_counter: _,
                        content_id_size: _,
                        name,
                    } => {
                        let value = self.values.remove(name).unwrap();
                        if let common::Value::Content(v) = value {
                            return Some((name.to_string(), Value::Content(v)));
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

impl BasicEntry {
    pub fn new_from_schema<VN: ToString, N:ToString>(
        schema: &schema::Schema,
        variant_name: Option<VN>,
        values: HashMap<N, common::Value>,
    ) -> Self {
        Self::new_from_schema_idx(schema, Default::default(), variant_name, values)
    }

    pub fn new_from_schema_idx<VN: ToString, N: ToString>(
        schema: &schema::Schema,
        idx: Vow<EntryIdx>,
        variant_name: Option<VN>,
        values: HashMap<N, common::Value>,
    ) -> Self {
        let variant_name = variant_name.map(|n| n.to_string());
        let value_transformer = ValueTransformer::new(schema, &variant_name, values
                        .into_iter()
                        .map(|(n, v)| (n.to_string(), v))
                        .collect());
        Self::new_idx(variant_name, value_transformer.collect(), idx)
    }

    pub fn new<VN: ToString, N: ToString>(variant_name: Option<VN>, values: HashMap<N, Value>) -> Self {
        Self::new_idx(variant_name, values, Default::default())
    }

    pub fn new_idx<VN: ToString, N: ToString>(
        variant_name: Option<VN>,
        values: HashMap<N, Value>,
        idx: Vow<EntryIdx>,
    ) -> Self {
        Self {
            variant_name: variant_name.map(|n| n.to_string()),
            values: values
                .into_iter()
                .map(|(n, v)| (n.to_string(), v))
                .collect(),
            idx,
        }
    }
}

impl EntryTrait for BasicEntry {
    fn variant_name(&self) -> Option<&str> {
        self.variant_name.as_deref()
    }
    fn value(&self, name: &str) -> &Value {
        &self.values[name]
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
    fn variant_name(&self) -> Option<&str> {
        T::variant_name(self)
    }
    fn value(&self, name: &str) -> &Value {
        T::value(self, name)
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
        sort_keys: &mut dyn Iterator<Item = &String>,
        other: &BasicEntry,
    ) -> cmp::Ordering {
        for property_name in sort_keys {
            let self_value = self.value(property_name);
            let other_value = other.value(property_name);
            match self_value.partial_cmp(other_value) {
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

impl<T> FullEntryTrait for Box<T>
where
    T: FullEntryTrait,
{
    fn compare(&self, sort_keys: &mut dyn Iterator<Item = &String>, other: &Self) -> cmp::Ordering {
        T::compare(self, sort_keys, other)
    }
}

struct Index {
    store_id: EntryStoreIdx,
    extra_data: ContentAddress,
    index_key: PropertyIdx,
    name: String,
    count: EntryCount,
    offset: Word<EntryIdx>,
}

impl Index {
    pub fn new(
        name: &str,
        extra_data: ContentAddress,
        index_key: PropertyIdx,
        store_id: EntryStoreIdx,
        count: EntryCount,
        offset: Word<EntryIdx>,
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
        self.offset.get().write(stream)?;
        self.extra_data.write(stream)?;
        self.index_key.write(stream)?;
        PString::write_string(self.name.as_ref(), stream)?;
        Ok(())
    }
}
