mod content_pack;
mod directory_pack;

use crate::bases::*;
use crate::pack::CheckKind;
pub use content_pack::ContentPackCreator;
pub use directory_pack::entry_def::{EntryDef as Entry, KeyDef as Key, VariantDef as Variant};
pub use directory_pack::{DirectoryPackCreator, Value};
use std::path::PathBuf;
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
    pub fn size(&self) -> u64 {
        match self.kind {
            CheckKind::None => 1,
            CheckKind::Blake3 => 33,
        }
    }
}

impl Writable for CheckInfo {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        self.kind.write(stream)?;
        stream.write_all(self.data.as_ref().unwrap())?;
        Ok(())
    }
}

pub struct PackInfo {
    pub uuid: uuid::Uuid,
    pub pack_id: u8,
    pub free_data: FreeData<U103>,
    pub pack_size: u64,
    pub pack_path: PathBuf,
    pub check_info: CheckInfo,
}

impl PackInfo {
    pub fn bytes(&self, check_info_pos: u64) -> Vec<u8> {
        let mut data = vec![];
        data.extend(self.uuid.as_bytes());
        data.push(self.pack_id);
        data.extend(&[0; 103]);
        data.extend(self.pack_size.to_be_bytes());
        data.extend(check_info_pos.to_be_bytes());
        data.extend(&[0; 8]); // offest
        let path_data = self.pack_path.as_os_str().to_str().unwrap().as_bytes();
        data.extend((path_data.len() as u8).to_be_bytes());
        data.extend(path_data);
        data.extend(vec![0; 256 - data.len()]);
        data
    }

    pub fn get_check_size(&self) -> u64 {
        self.check_info.size()
    }
}
