use super::{Embedded, PackData};
use crate::bases::*;
use crate::common::{ManifestPackHeader, PackHeaderInfo, PackInfo};
use std::fs::OpenOptions;
use std::io::{copy, repeat, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub struct ManifestPackCreator {
    app_vendor_id: u32,
    free_data: FreeData63,
    packs: Vec<PackData>,
    path: PathBuf,
}

impl ManifestPackCreator {
    pub fn new<P: AsRef<Path>>(path: P, app_vendor_id: u32, free_data: FreeData63) -> Self {
        ManifestPackCreator {
            app_vendor_id,
            free_data,
            packs: vec![],
            path: path.as_ref().into(),
        }
    }

    pub fn add_pack(&mut self, pack_info: PackData) {
        self.packs.push(pack_info);
    }

    pub fn finalize(self) -> IoResult<String> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        file.seek(SeekFrom::Start(128))?;

        let mut pack_infos = vec![];
        let nb_packs = self.packs.len() as u8;
        for pack_data in self.packs.into_iter() {
            let current_pos = file.tell();
            let sub_offset = match pack_data.embedded {
                Embedded::Yes => Offset::zero(),
                Embedded::No(_) => pack_data.check_info_pos,
            };
            copy(
                &mut pack_data.reader.create_stream_from(sub_offset),
                &mut file,
            )?;
            pack_infos.push(PackInfo::new_at_pos(pack_data, current_pos));
        }

        // Write the pack_info
        for pack_info in &pack_infos {
            pack_info.write(&mut file)?;
        }

        let check_offset = file.tell();
        let pack_size: Size = (check_offset + 33 + 64).into();

        file.rewind()?;
        let header = ManifestPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
            self.free_data,
            (nb_packs - 1).into(),
        );
        header.write(&mut file)?;
        file.rewind()?;

        let mut hasher = blake3::Hasher::new();
        let mut buf = [0u8; 256];
        file.read_exact(&mut buf[..128])?;
        hasher.write_all(&buf[..128])?; //check start
        for _i in 0..nb_packs {
            file.read_exact(&mut buf[..144])?;
            hasher.write_all(&buf[..144])?; //check beggining of pack
            copy(&mut repeat(0).take(112), &mut hasher)?; // fill with 0 the path
            file.seek(SeekFrom::Current(112))?;
        }
        copy(&mut file, &mut hasher)?; // finish
        let hash = hasher.finalize();
        file.write_u8(1)?;
        file.write_all(hash.as_bytes())?;

        file.rewind()?;
        let mut tail_buffer = [0u8; 64];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        Ok(self.path.to_str().unwrap().into())
    }
}
