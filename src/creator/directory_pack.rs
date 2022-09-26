pub mod entry_def;

use super::{CheckInfo, PackInfo};
use crate::bases::*;
use crate::common;
use crate::common::{Content, ContentAddress, DirectoryPackHeader, PackHeaderInfo};
use std::cell::RefCell;
use std::cmp;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use typenum::U31;

trait WritableTell {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()>;
    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()>;
    fn write(&self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
        self.write_data(stream)?;
        let offset = stream.tell();
        self.write_tail(stream)?;
        let size = stream.tell() - offset;
        Ok(SizedOffset { size, offset })
    }
}

#[derive(Debug)]
pub struct PlainKeyStore {
    idx: Idx<u8>,
    data: Vec<Vec<u8>>,
    size: usize,
}

impl PlainKeyStore {
    pub fn new(idx: Idx<u8>) -> Self {
        Self {
            idx,
            data: vec![],
            size: 0,
        }
    }

    pub fn add_key(&mut self, data: &[u8]) -> u64 {
        let offset = self.size as u64;
        self.data.push(data.to_vec());
        self.size += 1 + data.len();
        offset
    }

    pub fn key_size(&self) -> u16 {
        needed_bytes(self.size) as u16
    }

    pub fn get_idx(&self) -> Idx<u8> {
        self.idx
    }
}

impl WritableTell for PlainKeyStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for data in &self.data {
            PString::write_string(data, stream)?;
        }
        Ok(())
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        let size = Size(self.size as u64);
        size.write(stream)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct IndexedKeyStore {
    idx: Idx<u8>,
    data: Vec<Vec<u8>>,
    entries_offset: Vec<usize>,
}

impl IndexedKeyStore {
    pub fn new(idx: Idx<u8>) -> Self {
        Self {
            idx,
            data: vec![],
            entries_offset: vec![],
        }
    }

    fn current_offset(&self) -> usize {
        self.entries_offset.last().copied().unwrap_or(0)
    }

    pub fn add_key(&mut self, data: &[u8]) -> u64 {
        self.data.push(data.to_vec());
        self.entries_offset.push(self.current_offset() + data.len());
        (self.entries_offset.len() - 1) as u64
    }

    pub fn key_size(&self) -> u16 {
        needed_bytes(self.entries_offset.len()) as u16
    }

    pub fn get_idx(&self) -> Idx<u8> {
        self.idx
    }
}

impl WritableTell for IndexedKeyStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for d in &self.data {
            stream.write_data(d)?;
        }
        Ok(())
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        stream.write_u64(self.entries_offset.len() as u64)?; // key count
        let data_size = self.current_offset() as u64;
        let offset_size = needed_bytes(data_size);
        stream.write_u8(offset_size as u8)?; // offset_size
        stream.write_sized(data_size, offset_size)?; // data size
        for offset in &self.entries_offset[..(self.entries_offset.len() - 1)] {
            stream.write_sized(*offset as u64, offset_size)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum KeyStoreKind {
    Plain,
    Indexed,
}

#[derive(Debug)]
pub enum KeyStore {
    PlainKeyStore(PlainKeyStore),
    IndexedKeyStore(IndexedKeyStore),
}

impl KeyStore {
    pub fn new(kind: KeyStoreKind, idx: Idx<u8>) -> KeyStore {
        match kind {
            KeyStoreKind::Plain => KeyStore::PlainKeyStore(PlainKeyStore::new(idx)),
            KeyStoreKind::Indexed => KeyStore::IndexedKeyStore(IndexedKeyStore::new(idx)),
        }
    }

    pub fn add_key(&mut self, data: &[u8]) -> u64 {
        match self {
            KeyStore::PlainKeyStore(s) => s.add_key(data),
            KeyStore::IndexedKeyStore(s) => s.add_key(data),
        }
    }

    pub(crate) fn key_size(&self) -> u16 {
        match &self {
            KeyStore::PlainKeyStore(s) => s.key_size(),
            KeyStore::IndexedKeyStore(s) => s.key_size(),
        }
    }

    pub(crate) fn get_idx(&self) -> Idx<u8> {
        match &self {
            KeyStore::PlainKeyStore(s) => s.get_idx(),
            KeyStore::IndexedKeyStore(s) => s.get_idx(),
        }
    }
}

impl WritableTell for KeyStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        match &self {
            KeyStore::PlainKeyStore(s) => s.write_data(stream),
            KeyStore::IndexedKeyStore(s) => s.write_data(stream),
        }
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        match &self {
            KeyStore::PlainKeyStore(s) => {
                stream.write_u8(0x00)?;
                s.write_tail(stream)
            }
            KeyStore::IndexedKeyStore(s) => {
                stream.write_u8(0x01)?;
                s.write_tail(stream)
            }
        }
    }
}

enum Value {
    Content(Content),
    Unsigned(u64),
    Signed(i64),
    Array { data: Vec<u8>, key_id: Option<u64> },
}

pub struct Entry {
    variant_id: u8,
    values: Vec<Value>,
}

impl Entry {
    pub fn new(variant_id: u8, values: Vec<common::Value>) -> Self {
        let values = values
            .into_iter()
            .map(|v| match v {
                common::Value::Content(c) => Value::Content(c),
                common::Value::Unsigned(u) => Value::Unsigned(u),
                common::Value::Signed(s) => Value::Signed(s),
                common::Value::Array(a) => Value::Array {
                    data: a,
                    key_id: None,
                },
            })
            .collect();
        Self { variant_id, values }
    }
}

pub struct EntryStore {
    idx: Idx<u32>,
    entries: Vec<Entry>,
    entry_def: entry_def::EntryDef,
}

impl EntryStore {
    pub fn new(idx: Idx<u32>, entry_def: entry_def::EntryDef) -> Self {
        Self {
            idx,
            entries: vec![],
            entry_def,
        }
    }

    pub fn add_entry(&mut self, variant_id: u8, values: Vec<common::Value>) {
        self.entries.push(Entry::new(variant_id, values));
    }

    pub fn get_idx(&self) -> Idx<u32> {
        self.idx
    }

    fn finalize(&mut self) {
        for entry in &mut self.entries {
            let mut value_iter = entry.values.iter_mut();
            let variant = &mut self.entry_def.variants[entry.variant_id as usize];
            for key in &mut variant.keys {
                match key {
                    entry_def::KeyDef::PString(flookup_size, store_handle) => {
                        let flookup_size = *flookup_size;
                        let value = value_iter.next().unwrap();
                        if let Value::Array { data, key_id } = value {
                            *key_id =
                                Some(store_handle.borrow_mut().add_key(&data[flookup_size..]));
                            data.truncate(flookup_size);
                        }
                    }
                    entry_def::KeyDef::UnsignedInt(max_value) => {
                        let value = value_iter.next().unwrap();
                        if let Value::Unsigned(v) = value {
                            *key = entry_def::KeyDef::UnsignedInt(cmp::max(*max_value, *v));
                        }
                    }
                    _ => {
                        value_iter.next();
                    }
                }
            }
        }
        self.entry_def.finalize();
    }
}

impl WritableTell for EntryStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for entry in &self.entries {
            self.entry_def.write_entry(entry, stream)?;
        }
        Ok(())
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        stream.write_u8(0x00)?; // kind
        self.entry_def.write(stream)?;
        stream.write_u64((self.entries.len() * self.entry_def.entry_size() as usize) as u64)?;
        Ok(())
    }
}

struct Index {
    store_id: Idx<u32>,
    extra_data: ContentAddress,
    index_key: Idx<u8>,
    name: String,
    count: Count<u32>,
    offset: Idx<u32>,
}

impl Index {
    pub fn new(
        name: &str,
        extra_data: ContentAddress,
        index_key: Idx<u8>,
        store_id: Idx<u32>,
        count: Count<u32>,
        offset: Idx<u32>,
    ) -> Self {
        Index {
            store_id,
            extra_data,
            index_key,
            name: name.to_string(),
            count,
            offset,
        }
    }
}

impl WritableTell for Index {
    fn write_data(&self, _stream: &mut dyn OutStream) -> Result<()> {
        // No data to write
        Ok(())
    }
    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        self.store_id.write(stream)?;
        self.count.write(stream)?;
        self.offset.write(stream)?;
        self.extra_data.write(stream)?;
        self.index_key.write(stream)?;
        PString::write_string(self.name.as_ref(), stream)?;
        Ok(())
    }
}

pub struct DirectoryPackCreator {
    app_vendor_id: u32,
    pack_id: Id<u8>,
    free_data: FreeData<U31>,
    key_stores: Vec<Rc<RefCell<KeyStore>>>,
    entry_stores: Vec<EntryStore>,
    indexes: Vec<Index>,
    path: PathBuf,
}

impl DirectoryPackCreator {
    pub fn new<P: AsRef<Path>>(
        path: P,
        pack_id: Id<u8>,
        app_vendor_id: u32,
        free_data: FreeData<U31>,
    ) -> Self {
        DirectoryPackCreator {
            app_vendor_id,
            pack_id,
            free_data,
            key_stores: vec![],
            entry_stores: vec![],
            indexes: vec![],
            path: path.as_ref().into(),
        }
    }

    pub fn create_key_store(&mut self, kind: KeyStoreKind) -> Rc<RefCell<KeyStore>> {
        let idx = Idx::<u8>(self.key_stores.len() as u8);
        let key_store = Rc::new(RefCell::new(KeyStore::new(kind, idx)));
        self.key_stores.push(Rc::clone(&key_store));
        key_store
    }

    pub fn get_key_store(&mut self, idx: Idx<u8>) -> &RefCell<KeyStore> {
        &self.key_stores[idx.0 as usize]
    }

    pub fn create_entry_store(&mut self, entry_def: entry_def::EntryDef) -> Idx<u32> {
        let idx = Idx::<u32>(self.entry_stores.len() as u32);
        let entry_store = EntryStore::new(idx, entry_def);
        self.entry_stores.push(entry_store);
        idx
    }

    pub fn get_entry_store(&mut self, idx: Idx<u32>) -> &mut EntryStore {
        &mut self.entry_stores[idx.0 as usize]
    }

    pub fn create_index(
        &mut self,
        name: &str,
        extra_data: ContentAddress,
        index_key: Idx<u8>,
        store_id: Idx<u32>,
        count: Count<u32>,
        offset: Idx<u32>,
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
            128 + 8 * (self.key_stores.len() + self.entry_stores.len() + self.indexes.len());
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

        let mut key_stores_offsets = vec![];
        for key_store in &self.key_stores {
            key_stores_offsets.push(key_store.borrow().write(&mut file)?);
        }

        file.seek(SeekFrom::Start(128))?;
        let indexes_ptr_offsets = file.tell();
        for offset in &indexes_offsets {
            offset.write(&mut file)?;
        }
        let key_stores_ptr_offsets = file.tell();
        for offset in &key_stores_offsets {
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
            indexes_ptr_offsets,
            (indexes_offsets.len() as u32).into(),
            key_stores_ptr_offsets,
            (key_stores_offsets.len() as u8).into(),
            entry_stores_ptr_offsets,
            (entry_stores_offsets.len() as u32).into(),
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
            pack_size: pack_size.0,
            check_info: CheckInfo::new_blake3(hash.as_bytes()),
            pack_pos: self.path.clone().into(),
        })
    }
}
