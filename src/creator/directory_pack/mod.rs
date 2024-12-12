#[allow(clippy::module_inception)]
mod directory_pack;
mod entry_store;
mod layout;
pub mod schema;
mod value;
mod value_store;

use crate::bases::*;
use crate::common;
pub use directory_pack::DirectoryPackCreator;
pub use entry_store::EntryStore;
use std::cmp;
use std::collections::HashMap;
pub use value::{Array, ArrayS, Value};
pub(crate) use value_store::ValueStoreKind;
pub use value_store::{StoreHandle, ValueHandle, ValueStore};

pub trait PropertyName: ToString + std::cmp::Eq + std::hash::Hash + Copy + Send + 'static {}
impl PropertyName for &'static str {}

pub trait VariantName: ToString + std::cmp::Eq + std::hash::Hash + Copy + Send {}
impl VariantName for &str {}

pub trait EntryTrait<PN: PropertyName, VN: VariantName> {
    fn variant_name(&self) -> Option<MayRef<VN>>;
    fn value<'a>(&'a self, name: &PN) -> MayRef<'a, Value>;
    fn value_count(&self) -> PropertyCount;
    fn set_idx(&mut self, idx: EntryIdx);
    fn get_idx(&self) -> Bound<EntryIdx>;
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
pub struct BasicEntry<PN, VN> {
    variant_name: Option<VN>,
    names: Box<[PN]>,
    values: Box<[Value]>,
    idx: Vow<EntryIdx>,
}

pub(crate) struct ValueTransformer<'a, PN: PropertyName> {
    keys: Box<dyn Iterator<Item = &'a schema::Property<PN>> + 'a>,
    values: HashMap<PN, common::Value>,
}

impl<'a, PN: PropertyName> ValueTransformer<'a, PN> {
    pub fn new<VN: VariantName>(
        schema: &'a schema::Schema<PN, VN>,
        variant_name: &Option<VN>,
        values: HashMap<PN, common::Value>,
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
                variant_name.unwrap().to_string()
            );
        };
    }
}

impl<'a, PN: PropertyName> Iterator for ValueTransformer<'a, PN> {
    type Item = (PN, Value);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.keys.next() {
                None => return None,
                Some(key) => match key {
                    schema::Property::Array {
                        max_array_size: _,
                        fixed_array_len,
                        store_handle,
                        name,
                    } => {
                        let value = self
                            .values
                            .remove(name)
                            .unwrap_or_else(|| panic!("Cannot find entry {:?}", name.to_string()));
                        if let common::Value::Array(mut data) = value {
                            let size = data.len();
                            let to_store = data.split_off(cmp::min(*fixed_array_len, data.len()));
                            let value_id = store_handle.add_value(to_store);
                            return Some((
                                *name,
                                match data.len() {
                                    0 => Value::Array0(Box::new(ArrayS::<0> {
                                        size,
                                        value_id,
                                        data: data.try_into().unwrap(),
                                    })),
                                    1 => Value::Array1(Box::new(ArrayS::<1> {
                                        size,
                                        value_id,
                                        data: data.as_slice().try_into().unwrap(),
                                    })),
                                    2 => Value::Array2(Box::new(ArrayS::<2> {
                                        size,
                                        value_id,
                                        data: data.try_into().unwrap(),
                                    })),
                                    _ => Value::Array(Box::new(Array {
                                        size,
                                        data: data.into_boxed_slice(),
                                        value_id,
                                    })),
                                },
                            ));
                        } else {
                            panic!("Invalid value type");
                        }
                    }
                    schema::Property::IndirectArray { store_handle, name } => {
                        let value = self
                            .values
                            .remove(name)
                            .unwrap_or_else(|| panic!("Cannot find entry {:?}", name.to_string()));
                        if let common::Value::Array(data) = value {
                            let value_id = store_handle.add_value(data);
                            return Some((*name, Value::IndirectArray(Box::new(value_id))));
                        }
                    }
                    schema::Property::UnsignedInt {
                        counter: _,
                        size: _,
                        name,
                    } => match self.values.remove(name).unwrap() {
                        common::Value::Unsigned(v) => {
                            return Some((*name, Value::Unsigned(v)));
                        }
                        common::Value::UnsignedWord(v) => {
                            return Some((*name, Value::UnsignedWord(Box::new(v))));
                        }
                        _ => {
                            panic!("Invalid value type");
                        }
                    },
                    schema::Property::SignedInt {
                        counter: _,
                        size: _,
                        name,
                    } => match self.values.remove(name).unwrap() {
                        common::Value::Signed(v) => {
                            return Some((*name, Value::Signed(v)));
                        }
                        common::Value::SignedWord(v) => {
                            return Some((*name, Value::SignedWord(Box::new(v))));
                        }
                        _ => {
                            panic!("Invalid value type");
                        }
                    },
                    schema::Property::ContentAddress {
                        pack_id_counter: _,
                        pack_id_size: _,
                        content_id_size: _,
                        name,
                    } => {
                        let value = self.values.remove(name).unwrap();
                        if let common::Value::Content(v) = value {
                            return Some((*name, Value::Content(v)));
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

impl<PN: PropertyName, VN: VariantName> BasicEntry<PN, VN> {
    pub fn new_from_schema(
        schema: &schema::Schema<PN, VN>,
        variant_name: Option<VN>,
        values: HashMap<PN, common::Value>,
    ) -> Self {
        Self::new_from_schema_idx(schema, Default::default(), variant_name, values)
    }

    pub fn new_from_schema_idx(
        schema: &schema::Schema<PN, VN>,
        idx: Vow<EntryIdx>,
        variant_name: Option<VN>,
        values: HashMap<PN, common::Value>,
    ) -> Self {
        let value_transformer = ValueTransformer::<PN>::new(schema, &variant_name, values);
        Self::new_idx(variant_name, value_transformer.collect(), idx)
    }

    pub fn new(variant_name: Option<VN>, values: HashMap<PN, Value>) -> Self {
        Self::new_idx(variant_name, values, Default::default())
    }

    pub(crate) fn new_idx(
        variant_name: Option<VN>,
        values: HashMap<PN, Value>,
        idx: Vow<EntryIdx>,
    ) -> Self {
        let (names, values): (Vec<_>, Vec<_>) = values.into_iter().unzip();
        Self {
            variant_name,
            names: names.into(),
            values: values.into(),
            idx,
        }
    }
}

impl<PN: PropertyName, VN: VariantName> EntryTrait<PN, VN> for BasicEntry<PN, VN> {
    fn variant_name(&self) -> Option<MayRef<VN>> {
        self.variant_name.as_ref().map(MayRef::Borrowed)
    }
    fn value(&self, name: &PN) -> MayRef<Value> {
        match self.names.iter().position(|n| n == name) {
            Some(i) => MayRef::Borrowed(&self.values[i]),
            None => panic!("{} should be in entry", name.to_string()),
        }
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

impl<T, PN: PropertyName, VN: VariantName> EntryTrait<PN, VN> for Box<T>
where
    T: EntryTrait<PN, VN>,
{
    fn variant_name(&self) -> Option<MayRef<VN>> {
        T::variant_name(self)
    }
    fn value(&self, name: &PN) -> MayRef<Value> {
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

impl<PN: PropertyName, VN: VariantName> FullEntryTrait<PN, VN> for BasicEntry<PN, VN> {}

impl<T, PN: PropertyName, VN: VariantName> FullEntryTrait<PN, VN> for Box<T>
where
    T: FullEntryTrait<PN, VN>,
{
    fn compare<'i, I>(&self, sort_keys: &'i I, other: &Self) -> std::cmp::Ordering
    where
        I: IntoIterator<Item = &'i PN> + Copy,
    {
        T::compare(self, sort_keys, other)
    }
}

struct Index {
    store_id: EntryStoreIdx,
    free_data: IndexFreeData,
    index_key: PropertyIdx,
    name: String,
    count: EntryCount,
    offset: Word<EntryIdx>,
}

impl Index {
    pub fn new(
        name: &str,
        free_data: IndexFreeData,
        index_key: PropertyIdx,
        store_id: EntryStoreIdx,
        count: EntryCount,
        offset: Word<EntryIdx>,
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
        self.offset.get().serialize(ser)?;
        self.free_data.serialize(ser)?;
        self.index_key.serialize(ser)?;
        PString::serialize_string(self.name.as_ref(), ser)?;
        Ok(())
    }
}
