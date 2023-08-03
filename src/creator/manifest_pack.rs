use super::{Embedded, PackData};
use crate::bases::*;
use crate::common::{CheckInfo, ManifestCheckStream, ManifestPackHeader, PackHeaderInfo, PackInfo};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
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

    pub fn finalize(self) -> Result<String> {
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
            std::io::copy(
                &mut pack_data.reader.create_flux_from(sub_offset),
                &mut file,
            )?;
            pack_infos.push(PackInfo::new(
                pack_data,
                current_pos,
                self.path.parent().unwrap(),
            ));
        }

        let packs_offset = file.tell();
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

        let mut check_stream = ManifestCheckStream::new(&mut file, packs_offset, nb_packs.into());
        let check_info = CheckInfo::new_blake3(&mut check_stream)?;
        check_info.write(&mut file)?;

        file.rewind()?;
        let mut tail_buffer = [0u8; 64];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        Ok(self.path.to_str().unwrap().into())
    }
}
