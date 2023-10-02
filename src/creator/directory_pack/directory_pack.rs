use super::{entry_store, value_store, Index};
use crate::bases::*;
use crate::common::{CheckInfo, DirectoryPackHeader, PackHeaderInfo, PackKind};
use crate::creator::private::WritableTell;
use crate::creator::PackData;
use entry_store::EntryStoreTrait;
use std::io::{Read, Seek, SeekFrom, Write};
use value_store::ValueStore;

use log::info;

pub struct DirectoryPackCreator {
    app_vendor_id: u32,
    pack_id: PackId,
    free_data: DirectoryPackFreeData,
    value_stores: Vec<ValueStore>,
    entry_stores: Vec<Box<dyn EntryStoreTrait>>,
    indexes: Vec<Index>,
}

impl DirectoryPackCreator {
    pub fn new(pack_id: PackId, app_vendor_id: u32, free_data: DirectoryPackFreeData) -> Self {
        DirectoryPackCreator {
            app_vendor_id,
            pack_id,
            free_data,
            value_stores: vec![],
            entry_stores: vec![],
            indexes: vec![],
        }
    }

    pub fn add_value_store(&mut self, value_store: ValueStore) {
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

    pub fn finalize<O: Write + Read + Seek>(mut self, file: &mut O) -> Result<PackData> {
        let origin_offset = file.stream_position()?;
        let to_skip =
            128 + 8 * (self.value_stores.len() + self.entry_stores.len() + self.indexes.len());
        file.seek(SeekFrom::Current(to_skip as i64))?;

        info!("======= Finalize creation =======");

        info!("----- Finalize value_stores -----");
        for (idx, value_store) in &mut self.value_stores.iter().enumerate() {
            value_store
                .borrow_mut()
                .finalize(ValueStoreIdx::from(idx as u8));
        }

        info!("----- Finalize entry_stores -----");
        let finalized_entry_store: Vec<Box<dyn WritableTell>> = self
            .entry_stores
            .into_iter()
            .map(|e| e.finalize())
            .collect();

        info!("----- Write indexes -----");
        let mut indexes_offsets = vec![];
        for index in &mut self.indexes {
            indexes_offsets.push(index.write(file)?);
        }

        info!("----- Write entry_stores -----");
        let mut entry_stores_offsets = vec![];
        for mut entry_store in finalized_entry_store {
            entry_stores_offsets.push(entry_store.write(file)?);
        }

        info!("----- Write value_stores -----");
        let mut value_stores_offsets = vec![];
        for value_store in &self.value_stores {
            value_stores_offsets.push(value_store.borrow_mut().write(file)?);
        }

        file.seek(SeekFrom::Start(origin_offset + 128))?;
        info!("----- Write indexes offsets -----");
        let indexes_ptr_offsets = file.stream_position()? - origin_offset;
        for offset in &indexes_offsets {
            offset.write(file)?;
        }
        info!("----- Write value_stores offsets -----");
        let value_stores_ptr_offsets = file.stream_position()? - origin_offset;
        for offset in &value_stores_offsets {
            offset.write(file)?;
        }
        info!("----- Write entry_stores offsets -----");
        let entry_stores_ptr_offsets = file.stream_position()? - origin_offset;
        for offset in &entry_stores_offsets {
            offset.write(file)?;
        }

        info!("----- Write header -----");
        let check_offset = file.seek(SeekFrom::End(0))? - origin_offset;
        let pack_size: Size = (check_offset + 33 + 64).into();
        file.seek(SeekFrom::Start(origin_offset))?;
        let header = DirectoryPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset.into()),
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
        header.write(file)?;

        info!("----- Compute checksum -----");
        file.seek(SeekFrom::Start(origin_offset))?;
        let check_info = CheckInfo::new_blake3(file)?;
        check_info.write(file)?;

        file.seek(SeekFrom::Start(origin_offset))?;
        let mut tail_buffer = [0u8; 64];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        file.seek(SeekFrom::Start(origin_offset))?;
        Ok(PackData {
            uuid: header.uuid(),
            pack_id: self.pack_id,
            pack_kind: PackKind::Directory,
            free_data: Default::default(),
            pack_size,
            check_info,
        })
    }
}
