#[allow(clippy::module_inception)]
mod directory_pack;
mod entry_store;
pub mod layout;
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
    Array {
        data: Vec<u8>,
        value_id: Option<u64>,
    },
}

pub trait EntryTrait {
    fn variant_id(&self) -> Option<VariantIdx>;
    fn values(&self) -> Vec<Value>;
    fn finalize(&mut self, layout: &mut layout::Entry);
}

#[derive(Debug)]
pub struct BasicEntry {
    variant_id: Option<VariantIdx>,
    values: Vec<Value>,
}

impl BasicEntry {
    pub fn new(variant_id: Option<VariantIdx>, values: Vec<common::Value>) -> Self {
        let values = values
            .into_iter()
            .map(|v| match v {
                common::Value::Content(c) => Value::Content(c),
                common::Value::Unsigned(u) => Value::Unsigned(u),
                common::Value::Signed(s) => Value::Signed(s),
                common::Value::Array(a) => Value::Array {
                    data: a,
                    value_id: None,
                },
            })
            .collect();
        Self { variant_id, values }
    }

    fn finalize_keys<'a>(&mut self, mut keys: impl Iterator<Item = &'a mut layout::Property>) {
        let mut value_iter = self.values.iter_mut();
        for key in &mut keys {
            match key {
                layout::Property::VLArray(flookup_size, store_handle) => {
                    let flookup_size = *flookup_size;
                    let value = value_iter.next().unwrap();
                    if let Value::Array { data, value_id } = value {
                        let to_store = data.split_off(cmp::min(flookup_size, data.len()));
                        *value_id = Some(store_handle.borrow_mut().add_value(&to_store));
                    }
                }
                layout::Property::UnsignedInt(max_value) => {
                    let value = value_iter.next().unwrap();
                    if let Value::Unsigned(v) = value {
                        *key = layout::Property::UnsignedInt(cmp::max(*max_value, *v));
                    }
                }
                layout::Property::VariantId => {}
                _ => {
                    value_iter.next();
                }
            }
        }
    }
}

impl EntryTrait for BasicEntry {
    fn variant_id(&self) -> Option<VariantIdx> {
        self.variant_id
    }
    fn values(&self) -> Vec<Value> {
        self.values.clone()
    }

    fn finalize(&mut self, layout: &mut layout::Entry) {
        if layout.variants.is_empty() {
            self.finalize_keys(layout.common.iter_mut());
        } else {
            let keys = layout
                .common
                .iter_mut()
                .chain(layout.variants[self.variant_id.unwrap().into_usize()].iter_mut());
            self.finalize_keys(keys);
        }
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
