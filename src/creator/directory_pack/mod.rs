#[allow(clippy::module_inception)]
mod directory_pack;
pub mod entry_def;
mod entry_store;
mod value_store;

use super::{CheckInfo, PackInfo};
use crate::bases::*;
use crate::common;
use crate::common::{Content, ContentAddress};
pub use directory_pack::DirectoryPackCreator;
pub use value_store::ValueStoreKind;

trait WritableTell {
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

enum Value {
    Content(Content),
    Unsigned(u64),
    Signed(i64),
    Array {
        data: Vec<u8>,
        value_id: Option<u64>,
    },
}

pub struct Entry {
    variant_id: u8,
    values: Vec<Value>,
}

impl Entry {
    pub fn new(variant_id: u8, values: Vec<common::Value>) -> Self {
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
}

struct Index {
    store_id: Idx<u32>,
    extra_data: ContentAddress,
    index_key: Idx<u8>,
    name: String,
    count: Count<u32>,
    offset: Idx<u32>,
}

impl Index {
    pub fn new(
        name: &str,
        extra_data: ContentAddress,
        index_key: Idx<u8>,
        store_id: Idx<u32>,
        count: Count<u32>,
        offset: Idx<u32>,
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

impl WritableTell for Index {
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
