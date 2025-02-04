use camino::Utf8PathBuf;

use super::{private::WritableTell, PackData, StoreHandle, ValueStore};
use crate::bases::*;
use crate::common::{
    CheckInfo, CheckKind, ManifestCheckStream, ManifestPackHeader, PackHeader, PackHeaderInfo,
    PackInfo, PackKind,
};
use crate::creator::Result;
use std::io::SeekFrom;

pub struct ManifestPackCreator {
    app_vendor_id: VendorId,
    free_data: PackFreeData,
    packs: Vec<(PackData, Utf8PathBuf)>,
    value_store: StoreHandle,
}

impl ManifestPackCreator {
    pub fn new(app_vendor_id: VendorId, free_data: PackFreeData) -> Self {
        ManifestPackCreator {
            app_vendor_id,
            free_data,
            packs: vec![],
            value_store: ValueStore::new_indexed(),
        }
    }

    pub fn add_pack(&mut self, pack_info: PackData, locator: impl Into<Utf8PathBuf>) {
        self.packs.push((pack_info, locator.into()));
    }

    pub fn finalize<O: InOutStream>(self, file: &mut O) -> Result<uuid::Uuid> {
        let origin_offset = file.stream_position()?;
        file.seek(SeekFrom::Current(
            (PackHeader::BLOCK_SIZE + ManifestPackHeader::BLOCK_SIZE) as i64,
        ))?;

        let mut pack_infos = vec![];
        let mut free_data_ids = vec![];

        let nb_packs = self.packs.len() as u16;

        for (pack_data, _locator) in &self.packs {
            let free_data_id = self.value_store.add_value(pack_data.free_data.to_vec());
            free_data_ids.push(free_data_id);
        }

        self.value_store.finalize(0.into());

        for ((pack_data, locator), free_data_id) in self.packs.into_iter().zip(free_data_ids) {
            let check_info_pos = file.stream_position()? - origin_offset;
            file.ser_write(&pack_data.check_info)?;
            let check_info_size = file.stream_position()? - origin_offset - check_info_pos;
            let check_info_size: usize = check_info_size.try_into().unwrap();
            pack_infos.push(PackInfo::new(
                pack_data,
                0,
                free_data_id.get(),
                SizedOffset::new(check_info_size.into(), check_info_pos.into()),
                locator.into(),
            ));
        }

        let value_store_pos = self.value_store.write().unwrap().write(file)?;

        let packs_offset = file.stream_position()? - origin_offset;
        // Write the pack_info
        for pack_info in &pack_infos {
            file.ser_write(pack_info)?;
        }

        let check_offset = file.stream_position()? - origin_offset;
        let pack_size: Size = (check_offset
            + CheckKind::Blake3.block_size().into_u64()
            + PackHeader::BLOCK_SIZE as u64)
            .into();

        file.seek(SeekFrom::Start(origin_offset))?;

        let pack_header = PackHeader::new(
            PackKind::Manifest,
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset.into()),
        );
        file.ser_write(&pack_header)?;
        let header = ManifestPackHeader::new(self.free_data, nb_packs.into(), value_store_pos);
        file.ser_write(&header)?;
        file.seek(SeekFrom::Start(origin_offset))?;

        let mut check_stream = ManifestCheckStream::new(file, packs_offset.into(), nb_packs.into());
        let check_info = CheckInfo::new_blake3(&mut check_stream)?;
        file.ser_write(&check_info)?;

        file.seek(SeekFrom::Start(origin_offset))?;
        let mut tail_buffer = [0u8; PackHeader::BLOCK_SIZE];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        Ok(pack_header.uuid)
    }
}
