mod content_pack;
mod directory_pack;
mod manifest_pack;

use crate::bases::*;
pub use crate::bases::{BufStream, FileStream};
use crate::common::{CheckKind, PackPos};
pub use crate::common::{Content, Value};
pub use content_pack::ContentPackCreator;
pub use directory_pack::entry_def::{EntryDef as Entry, KeyDef as Key, VariantDef as Variant};
pub use directory_pack::{DirectoryPackCreator, KeyStoreKind};
pub use manifest_pack::ManifestPackCreator;
use typenum::U103;

pub struct CheckInfo {
    kind: CheckKind,
    data: Option<Vec<u8>>,
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
            CheckKind::None => Size(1),
            CheckKind::Blake3 => Size(33),
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

pub struct PackInfo {
    pub uuid: uuid::Uuid,
    pub pack_id: Id<u8>,
    pub free_data: FreeData<U103>,
    pub pack_size: u64,
    pub pack_pos: PackPos,
    pub check_info: CheckInfo,
}

impl PackInfo {
    pub fn write(&self, check_info_pos: Offset, stream: &mut dyn OutStream) -> IoResult<()> {
        self.uuid.write(stream)?;
        self.pack_id.write(stream)?;
        self.free_data.write(stream)?;
        stream.write_u64(self.pack_size)?;
        check_info_pos.write(stream)?;
        match &self.pack_pos {
            PackPos::Offset(offset) => {
                offset.write(stream)?;
                PString::write_string_padded(b"", 111, stream)?;
            }
            PackPos::Path(path) => {
                stream.write_u64(0)?;
                PString::write_string_padded(path.as_ref(), 111, stream)?;
            }
        }
        Ok(())
    }

    pub fn get_check_size(&self) -> Size {
        self.check_info.size()
    }
}
