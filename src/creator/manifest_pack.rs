use super::{private::WritableTell, PackData, StoreHandle, ValueStore};
use crate::bases::*;
use crate::common::{CheckInfo, ManifestCheckStream, ManifestPackHeader, PackHeaderInfo, PackInfo};
use std::io::{Read, SeekFrom};

pub struct ManifestPackCreator {
    app_vendor_id: u32,
    free_data: ManifestPackFreeData,
    packs: Vec<(PackData, Vec<u8>)>,
    value_store: StoreHandle,
}

impl ManifestPackCreator {
    pub fn new(app_vendor_id: u32, free_data: ManifestPackFreeData) -> Self {
        ManifestPackCreator {
            app_vendor_id,
            free_data,
            packs: vec![],
            value_store: ValueStore::new_indexed(),
        }
    }

    pub fn add_pack(&mut self, pack_info: PackData, locator: Vec<u8>) {
        self.packs.push((pack_info, locator));
    }

    pub fn finalize<O: OutStream + Read>(self, file: &mut O) -> Result<uuid::Uuid> {
        let origin_offset = file.stream_position()?;
        file.seek(SeekFrom::Current(128))?;

        let mut pack_infos = vec![];
        let mut free_data_ids = vec![];

        let nb_packs = self.packs.len() as u16;

        for (pack_data, _locator) in &self.packs {
            let free_data_id = self
                .value_store
                .borrow_mut()
                .add_value(pack_data.free_data.as_slice());
            free_data_ids.push(free_data_id);
        }

        self.value_store.borrow_mut().finalize(0.into());

        for ((pack_data, locator), free_data_id) in self.packs.into_iter().zip(free_data_ids) {
            let current_pos = file.stream_position()? - origin_offset;
            pack_data.check_info.write(file)?;
            pack_infos.push(PackInfo::new(
                pack_data,
                0,
                free_data_id.get().into(),
                current_pos.into(),
                locator,
            ));
        }

        let value_store_pos = self.value_store.borrow_mut().write(file)?;

        let packs_offset = file.stream_position()? - origin_offset;
        // Write the pack_info
        for pack_info in &pack_infos {
            pack_info.write(file)?;
        }

        let check_offset = file.stream_position()? - origin_offset;
        let pack_size: Size = (check_offset + 33 + 64).into();

        file.seek(SeekFrom::Start(origin_offset))?;
        let header = ManifestPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset.into()),
            self.free_data,
            nb_packs.into(),
            value_store_pos,
        );
        header.write(file)?;
        file.seek(SeekFrom::Start(origin_offset))?;

        let mut check_stream = ManifestCheckStream::new(file, packs_offset.into(), nb_packs.into());
        let check_info = CheckInfo::new_blake3(&mut check_stream)?;
        check_info.write(file)?;

        file.seek(SeekFrom::Start(origin_offset))?;
        let mut tail_buffer = [0u8; 64];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        Ok(header.pack_header.uuid)
    }
}
