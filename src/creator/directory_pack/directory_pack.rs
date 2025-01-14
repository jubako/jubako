use super::{entry_store, value_store, Index};
use crate::bases::*;
use crate::common::{
    CheckInfo, CheckKind, DirectoryPackHeader, PackHeader, PackHeaderInfo, PackKind,
};
use crate::creator::private::WritableTell;
use crate::creator::{PackData, Result};
use entry_store::EntryStoreTrait;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use value_store::StoreHandle;

use log::info;

pub struct DirectoryPackCreator {
    app_vendor_id: VendorId,
    pack_id: PackId,
    free_data: PackFreeData,
    value_stores: Vec<StoreHandle>,
    entry_stores: Vec<Box<dyn EntryStoreTrait>>,
    indexes: Vec<Index>,
}

impl DirectoryPackCreator {
    pub fn new(pack_id: PackId, app_vendor_id: VendorId, free_data: PackFreeData) -> Self {
        DirectoryPackCreator {
            app_vendor_id,
            pack_id,
            free_data,
            value_stores: vec![],
            entry_stores: vec![],
            indexes: vec![],
        }
    }

    pub fn add_value_store(&mut self, value_store: StoreHandle) {
        self.value_stores.push(value_store);
    }

    pub fn add_entry_store(&mut self, entry_store: Box<dyn EntryStoreTrait>) -> EntryStoreIdx {
        let idx = (self.entry_stores.len() as u32).into();
        self.entry_stores.push(entry_store);
        idx
    }

    pub fn create_index(
        &mut self,
        name: &str,
        free_data: IndexFreeData,
        index_key: PropertyIdx,
        store_id: EntryStoreIdx,
        count: EntryCount,
        offset: Word<EntryIdx>,
    ) {
        let index = Index::new(name, free_data, index_key, store_id, count, offset);
        self.indexes.push(index);
    }

    pub fn finalize(self) -> std::io::Result<FinalizedDirectoryPackCreator> {
        info!("======= Finalize creation =======");

        info!("----- Finalize value_stores -----");
        for (idx, value_store) in &mut self.value_stores.iter().enumerate() {
            value_store.finalize(ValueStoreIdx::from(idx as u8));
        }

        info!("----- Finalize entry_stores -----");
        let finalized_entry_stores: Vec<Box<dyn WritableTell>> = self
            .entry_stores
            .into_iter()
            .map(|e| e.finalize())
            .collect();

        Ok(FinalizedDirectoryPackCreator {
            app_vendor_id: self.app_vendor_id,
            pack_id: self.pack_id,
            free_data: self.free_data,
            value_stores: self.value_stores,
            entry_stores: finalized_entry_stores,
            indexes: self.indexes,
        })
    }
}

pub struct FinalizedDirectoryPackCreator {
    app_vendor_id: VendorId,
    pack_id: PackId,
    free_data: PackFreeData,
    value_stores: Vec<StoreHandle>,
    entry_stores: Vec<Box<dyn WritableTell>>,
    indexes: Vec<Index>,
}

impl FinalizedDirectoryPackCreator {
    pub fn write<O: InOutStream>(mut self, file: &mut O) -> Result<PackData> {
        let origin_offset = file.stream_position()?;
        let to_skip = PackHeader::BLOCK_SIZE + DirectoryPackHeader::BLOCK_SIZE;
        file.seek(SeekFrom::Current(to_skip as i64))?;

        let mut buffered = BufWriter::new(file);

        info!("----- Write indexes -----");
        let mut indexes_offsets = vec![];
        for index in &mut self.indexes {
            indexes_offsets.push(index.write(&mut buffered)?);
        }

        info!("----- Write entry_stores -----");
        let mut entry_stores_offsets = vec![];
        for mut entry_store in self.entry_stores {
            entry_stores_offsets.push(entry_store.write(&mut buffered)?);
        }

        info!("----- Write value_stores -----");
        let mut value_stores_offsets = vec![];
        for value_store in &self.value_stores {
            value_stores_offsets.push(value_store.write().unwrap().write(&mut buffered)?);
        }

        info!("----- Write indexes offsets -----");
        let indexes_ptr_offsets = buffered.stream_position()? - origin_offset;
        buffered.ser_callable(&|ser| {
            for offset in &indexes_offsets {
                offset.serialize(ser)?;
            }
            Ok(())
        })?;

        info!("----- Write value_stores offsets -----");
        let value_stores_ptr_offsets = buffered.stream_position()? - origin_offset;
        buffered.ser_callable(&|ser| {
            for offset in &value_stores_offsets {
                offset.serialize(ser)?;
            }
            Ok(())
        })?;

        info!("----- Write entry_stores offsets -----");
        let entry_stores_ptr_offsets = buffered.stream_position()? - origin_offset;
        buffered.ser_callable(&|ser| {
            for offset in &entry_stores_offsets {
                offset.serialize(ser)?;
            }
            Ok(())
        })?;

        buffered.flush()?;
        let file = buffered.into_inner().unwrap();

        let check_offset = file.seek(SeekFrom::End(0))? - origin_offset;
        let pack_size: Size = (check_offset
            + CheckKind::Blake3.block_size().into_u64()
            + PackHeader::BLOCK_SIZE as u64)
            .into();
        file.seek(SeekFrom::Start(origin_offset))?;

        info!("----- Write pack header -----");
        let pack_header = PackHeader::new(
            PackKind::Directory,
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset.into()),
        );
        file.ser_write(&pack_header)?;

        info!("----- Write directory pack header -----");
        let header = DirectoryPackHeader::new(
            self.free_data,
            (
                (indexes_offsets.len() as u32).into(),
                indexes_ptr_offsets.into(),
            ),
            (
                (value_stores_offsets.len() as u8).into(),
                value_stores_ptr_offsets.into(),
            ),
            (
                (entry_stores_offsets.len() as u32).into(),
                entry_stores_ptr_offsets.into(),
            ),
        );
        file.ser_write(&header)?;

        info!("----- Compute checksum -----");
        file.seek(SeekFrom::Start(origin_offset))?;
        let check_info = CheckInfo::new_blake3(file)?;
        file.ser_write(&check_info)?;

        file.seek(SeekFrom::Start(origin_offset))?;
        let mut tail_buffer = [0u8; PackHeader::BLOCK_SIZE];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        file.seek(SeekFrom::Start(origin_offset))?;
        Ok(PackData {
            uuid: pack_header.uuid,
            pack_id: self.pack_id,
            pack_kind: PackKind::Directory,
            free_data: Default::default(),
            pack_size,
            check_info,
        })
    }
}
