use super::{entry_store, value_store, Index};
use crate::bases::*;
use crate::common::{ContentAddress, DirectoryPackHeader, PackHeaderInfo};
use crate::creator::private::WritableTell;
use crate::creator::{Embedded, PackData};
use entry_store::EntryStoreTrait;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use value_store::ValueStore;

use log::info;

pub struct DirectoryPackCreator {
    app_vendor_id: u32,
    pack_id: PackId,
    free_data: FreeData31,
    value_stores: Vec<ValueStore>,
    entry_stores: Vec<Box<dyn EntryStoreTrait>>,
    indexes: Vec<Index>,
    path: PathBuf,
}

impl DirectoryPackCreator {
    pub fn new<P: AsRef<Path>>(
        path: P,
        pack_id: PackId,
        app_vendor_id: u32,
        free_data: FreeData31,
    ) -> Self {
        DirectoryPackCreator {
            app_vendor_id,
            pack_id,
            free_data,
            value_stores: vec![],
            entry_stores: vec![],
            indexes: vec![],
            path: path.as_ref().into(),
        }
    }

    pub fn add_value_store(&mut self, value_store: ValueStore) {
        self.value_stores.push(value_store);
    }

    pub fn add_entry_store(&mut self, mut entry_store: Box<dyn EntryStoreTrait>) -> EntryStoreIdx {
        let idx = (self.entry_stores.len() as u32).into();
        entry_store.set_idx(idx);
        self.entry_stores.push(entry_store);
        idx
    }

    pub fn create_index(
        &mut self,
        name: &str,
        extra_data: ContentAddress,
        index_key: PropertyIdx,
        store_id: EntryStoreIdx,
        count: EntryCount,
        offset: Word<EntryIdx>,
    ) {
        let index = Index::new(name, extra_data, index_key, store_id, count, offset);
        self.indexes.push(index);
    }

    pub fn finalize(mut self, path: Option<PathBuf>) -> Result<PackData> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        let to_skip =
            128 + 8 * (self.value_stores.len() + self.entry_stores.len() + self.indexes.len());
        file.seek(SeekFrom::Start(to_skip as u64))?;

        info!("======= Finalize creation =======");

        info!("----- Finalize value_stores -----");
        for (idx, value_store) in &mut self.value_stores.iter().enumerate() {
            value_store
                .borrow_mut()
                .finalize(ValueStoreIdx::from(idx as u8));
        }

        info!("----- Finalize entry_stores -----");
        for entry_store in &mut self.entry_stores {
            entry_store.finalize();
        }

        info!("----- Write indexes -----");
        let mut indexes_offsets = vec![];
        for index in &mut self.indexes {
            indexes_offsets.push(index.write(&mut file)?);
        }

        info!("----- Write entry_stores -----");
        let mut entry_stores_offsets = vec![];
        for entry_store in &mut self.entry_stores {
            entry_stores_offsets.push(entry_store.write(&mut file)?);
        }

        info!("----- Write value_stores -----");
        let mut value_stores_offsets = vec![];
        for value_store in &self.value_stores {
            value_stores_offsets.push(value_store.borrow_mut().write(&mut file)?);
        }

        file.seek(SeekFrom::Start(128))?;
        info!("----- Write indexes offsets -----");
        let indexes_ptr_offsets = file.tell();
        for offset in &indexes_offsets {
            offset.write(&mut file)?;
        }
        info!("----- Write value_stores offsets -----");
        let value_stores_ptr_offsets = file.tell();
        for offset in &value_stores_offsets {
            offset.write(&mut file)?;
        }
        info!("----- Write entry_stores offsets -----");
        let entry_stores_ptr_offsets = file.tell();
        for offset in &entry_stores_offsets {
            offset.write(&mut file)?;
        }

        info!("----- Write header -----");
        file.seek(SeekFrom::End(0))?;
        let check_offset = file.tell();
        let pack_size: Size = (check_offset + 33 + 64).into();
        file.rewind()?;
        let header = DirectoryPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
            self.free_data,
            ((indexes_offsets.len() as u32).into(), indexes_ptr_offsets),
            (
                (value_stores_offsets.len() as u8).into(),
                value_stores_ptr_offsets,
            ),
            (
                (entry_stores_offsets.len() as u32).into(),
                entry_stores_ptr_offsets,
            ),
        );
        header.write(&mut file)?;

        info!("----- Compute checksum -----");
        file.rewind()?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_u8(1)?;
        file.write_all(hash.as_bytes())?;

        file.rewind()?;
        let mut tail_buffer = [0u8; 64];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        file.rewind()?;
        Ok(PackData {
            uuid: header.uuid(),
            pack_id: self.pack_id,
            free_data: FreeData103::clone_from_slice(&[0; 103]),
            reader: FileSource::new(file)?.into(),
            check_info_pos: check_offset,
            embedded: match path {
                None => Embedded::Yes,
                Some(p) => Embedded::No(p),
            },
        })
    }
}
