mod content_pack;
mod directory_pack;
mod manifest_pack;

use crate::bases::*;
pub use crate::bases::{BufStream, FileStream};
use crate::common::{CheckKind, PackPos};
pub use crate::common::{Content, Value};
pub use content_pack::ContentPackCreator;
pub use directory_pack::{layout, DirectoryPackCreator, ValueStoreKind};
pub use manifest_pack::ManifestPackCreator;

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

pub struct PackInfo {
    pub uuid: uuid::Uuid,
    pub pack_id: PackId,
    pub free_data: FreeData103,
    pub pack_size: Size,
    pub pack_pos: PackPos,
    pub check_info: CheckInfo,
}

impl PackInfo {
    pub fn write(&self, check_info_pos: Offset, stream: &mut dyn OutStream) -> IoResult<()> {
        self.uuid.write(stream)?;
        self.pack_id.write(stream)?;
        self.free_data.write(stream)?;
        self.pack_size.write(stream)?;
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
