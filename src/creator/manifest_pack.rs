use super::PackInfo;
use crate::bases::*;
use crate::common::{ManifestPackHeader, PackHeaderInfo};
use std::fs::OpenOptions;
use std::io::{copy, repeat, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use typenum::U63;

pub struct ManifestPackCreator {
    app_vendor_id: u32,
    free_data: FreeData<U63>,
    packs: Vec<PackInfo>,
    path: PathBuf,
}

impl ManifestPackCreator {
    pub fn new<P: AsRef<Path>>(path: P, app_vendor_id: u32, free_data: FreeData<U63>) -> Self {
        ManifestPackCreator {
            app_vendor_id,
            free_data,
            packs: vec![],
            path: path.as_ref().into(),
        }
    }

    pub fn add_pack(&mut self, pack_info: PackInfo) {
        self.packs.push(pack_info);
    }

    pub fn finalize(&mut self) -> IoResult<String> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        file.seek(SeekFrom::Start(128))?;

        let mut check_pos = Offset(128 + 256 * (self.packs.len() as u64));

        for pack_info in &self.packs {
            pack_info.write(check_pos, &mut file)?;
            check_pos += pack_info.get_check_size();
        }

        for pack_info in &self.packs {
            pack_info.check_info.write(&mut file)?;
        }

        let check_offset = file.tell();
        let pack_size: Size = (check_offset + 33).into();
        file.rewind()?;
        let header = ManifestPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
            self.free_data,
            ((self.packs.len() as u8) - 1).into(),
        );
        header.write(&mut file)?;
        file.rewind()?;

        let mut hasher = blake3::Hasher::new();
        let mut buf = [0u8; 256];
        file.read_exact(&mut buf[..128])?;
        hasher.write_all(&buf[..128])?; //check start
        for _i in 0..self.packs.len() {
            file.read_exact(&mut buf[..144])?;
            hasher.write_all(&buf[..144])?; //check beggining of pack
            copy(&mut repeat(0).take(112), &mut hasher)?; // fill with 0 the path
            file.seek(SeekFrom::Current(112))?;
        }
        copy(&mut file, &mut hasher)?; // finish
        let hash = hasher.finalize();
        file.write_u8(1)?;
        file.write_all(hash.as_bytes())?;
        Ok(self.path.to_str().unwrap().into())
    }
}
