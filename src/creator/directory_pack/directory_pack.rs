use super::{entry_store, layout, value_store, CheckInfo, Index, PackInfo, WritableTell};
use crate::bases::*;
use crate::common::{ContentAddress, DirectoryPackHeader, PackHeaderInfo};
use entry_store::EntryStore;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use typenum::U31;
use value_store::ValueStore;
pub use value_store::ValueStoreKind;

pub struct DirectoryPackCreator {
    app_vendor_id: u32,
    pack_id: PackId,
    free_data: FreeData<U31>,
    value_stores: Vec<Rc<RefCell<ValueStore>>>,
    entry_stores: Vec<EntryStore>,
    indexes: Vec<Index>,
    path: PathBuf,
}

impl DirectoryPackCreator {
    pub fn new<P: AsRef<Path>>(
        path: P,
        pack_id: PackId,
        app_vendor_id: u32,
        free_data: FreeData<U31>,
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

    pub fn create_value_store(&mut self, kind: ValueStoreKind) -> Rc<RefCell<ValueStore>> {
        let idx = ValueStoreIdx::from(self.value_stores.len() as u8);
        let value_store = Rc::new(RefCell::new(ValueStore::new(kind, idx)));
        self.value_stores.push(Rc::clone(&value_store));
        value_store
    }

    pub fn get_value_store(&mut self, idx: ValueStoreIdx) -> &RefCell<ValueStore> {
        &self.value_stores[idx.into_usize()]
    }

    pub fn create_entry_store(&mut self, layout: layout::Entry) -> EntryStoreIdx {
        let idx = (self.entry_stores.len() as u32).into();
        let entry_store = EntryStore::new(idx, layout);
        self.entry_stores.push(entry_store);
        idx
    }

    pub fn get_entry_store(&mut self, idx: EntryStoreIdx) -> &mut EntryStore {
        &mut self.entry_stores[idx.into_usize()]
    }

    pub fn create_index(
        &mut self,
        name: &str,
        extra_data: ContentAddress,
        index_key: PropertyIdx,
        store_id: EntryStoreIdx,
        count: EntryCount,
        offset: EntryIdx,
    ) {
        let index = Index::new(name, extra_data, index_key, store_id, count, offset);
        self.indexes.push(index);
    }

    pub fn finalize(&mut self) -> Result<PackInfo> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        let to_skip =
            128 + 8 * (self.value_stores.len() + self.entry_stores.len() + self.indexes.len());
        file.seek(SeekFrom::Start(to_skip as u64))?;

        for entry_store in &mut self.entry_stores {
            entry_store.finalize();
        }

        let mut indexes_offsets = vec![];
        for index in &self.indexes {
            indexes_offsets.push(index.write(&mut file)?);
        }

        let mut entry_stores_offsets = vec![];
        for entry_store in &self.entry_stores {
            entry_stores_offsets.push(entry_store.write(&mut file)?);
        }

        let mut value_stores_offsets = vec![];
        for value_store in &self.value_stores {
            value_stores_offsets.push(value_store.borrow().write(&mut file)?);
        }

        file.seek(SeekFrom::Start(128))?;
        let indexes_ptr_offsets = file.tell();
        for offset in &indexes_offsets {
            offset.write(&mut file)?;
        }
        let value_stores_ptr_offsets = file.tell();
        for offset in &value_stores_offsets {
            offset.write(&mut file)?;
        }
        let entry_stores_ptr_offsets = file.tell();
        for offset in &entry_stores_offsets {
            offset.write(&mut file)?;
        }

        file.seek(SeekFrom::End(0))?;
        let check_offset = file.tell();
        let pack_size: Size = (check_offset + 33).into();
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
        file.rewind()?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_u8(1)?;
        file.write_all(hash.as_bytes())?;
        Ok(PackInfo {
            uuid: header.uuid(),
            pack_id: self.pack_id,
            free_data: FreeData::clone_from_slice(&[0; 103]),
            pack_size,
            check_info: CheckInfo::new_blake3(hash.as_bytes()),
            pack_pos: self.path.clone().into(),
        })
    }
}
