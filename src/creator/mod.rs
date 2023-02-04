mod content_pack;
mod directory_pack;
mod manifest_pack;

use crate::bases::*;
pub use crate::bases::{FileSource, Stream};
use crate::common::CheckKind;
pub use crate::common::Value;
pub use content_pack::ContentPackCreator;
pub use directory_pack::{
    schema, BasicEntry, DirectoryPackCreator, EntryStore, EntryTrait, ValueStoreKind,
    ValueTransformer,
};
pub use manifest_pack::ManifestPackCreator;
use std::path::PathBuf;

pub struct CheckInfo {
    kind: CheckKind,
    data: Option<Vec<u8>>,
}

pub enum Embedded {
    Yes,
    No(PathBuf),
}
impl CheckInfo {
    pub fn new_blake3(hash: &[u8]) -> Self {
        Self {
            kind: CheckKind::Blake3,
            data: Some(hash.to_vec()),
        }
    }
    pub fn size(&self) -> Size {
        match self.kind {
            CheckKind::None => Size::new(1),
            CheckKind::Blake3 => Size::new(33),
        }
    }
}

impl Writable for CheckInfo {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += self.kind.write(stream)?;
        written += stream.write_data(self.data.as_ref().unwrap())?;
        Ok(written)
    }
}

mod private {
    use super::*;
    pub trait WritableTell {
        fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<()>;
        fn write_tail(&mut self, stream: &mut dyn OutStream) -> Result<()>;
        fn write(&mut self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
            self.write_data(stream)?;
            let offset = stream.tell();
            self.write_tail(stream)?;
            let size = stream.tell() - offset;
            Ok(SizedOffset { size, offset })
        }
    }
}

pub struct PackData {
    pub uuid: uuid::Uuid,
    pub pack_id: PackId,
    pub free_data: FreeData103,
    pub reader: Reader,
    pub check_info_pos: Offset,
    pub embedded: Embedded,
}
