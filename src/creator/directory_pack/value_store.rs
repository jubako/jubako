use crate::bases::*;
use crate::creator::private::WritableTell;
use rayon::prelude::*;
use std::cell::Cell;

use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueStoreKind {
    Plain,
    Indexed,
}

pub struct ValueHandle {
    store: Cell<Option<StoreHandle>>,
    idx: Cell<u64>,
}

impl std::fmt::Debug for ValueHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("ValueHandle")
            .field("store", &"Unknown")
            .field("idx", &self.idx.get())
            .finish()
    }
}

impl ValueHandle {
    fn new(store: &Arc<RwLock<ValueStore>>, idx: usize) -> Self {
        Self {
            store: Cell::new(Some(StoreHandle(Arc::clone(store)))),
            idx: Cell::new(idx as u64),
        }
    }

    pub fn get(&self) -> ValueIdx {
        match self.store.take() {
            None => self.idx.get().into(),
            Some(store) => {
                let idx = store.get(self.idx.get() as usize);
                self.idx.set(idx.into_u64());
                idx
            }
        }
    }

    pub fn clone_get(&self) -> Self {
        let idx = self.get();
        Self {
            store: Cell::new(None),
            idx: Cell::new(idx.into()),
        }
    }
}

impl PartialEq for ValueHandle {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl From<ValueHandle> for Word<u64> {
    fn from(value_handle: ValueHandle) -> Self {
        let func: Box<dyn Fn() -> u64 + Sync + Send> = match value_handle.store.take() {
            None => {
                let idx = value_handle.idx.get();
                Box::new(move || idx)
            }
            Some(store) => {
                let idx = value_handle.idx.get() as usize;
                Box::new(move || store.get(idx).into_u64())
            }
        };
        func.into()
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct StoreHandle(Arc<RwLock<ValueStore>>);

impl StoreHandle {
    pub(crate) fn key_size(&self) -> ByteSize {
        self.0.read().unwrap().key_size()
    }

    pub(crate) fn get_idx(&self) -> Option<ValueStoreIdx> {
        self.0.read().unwrap().get_idx()
    }

    pub fn add_value(&self, data: Vec<u8>) -> ValueHandle {
        let idx = self.0.write().unwrap().add_value(data);
        ValueHandle::new(&self.0, idx)
    }

    pub(crate) fn finalize(&self, idx: ValueStoreIdx) {
        self.0.write().unwrap().finalize(idx)
    }

    pub(crate) fn kind(&self) -> ValueStoreKind {
        self.0.read().unwrap().kind()
    }

    fn get(&self, idx: usize) -> ValueIdx {
        self.0.read().unwrap().get(idx)
    }
}

impl From<ValueStore> for StoreHandle {
    fn from(s: ValueStore) -> Self {
        Self(Arc::new(RwLock::new(s)))
    }
}

impl std::ops::Deref for StoreHandle {
    type Target = Arc<RwLock<ValueStore>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum ValueStore {
    Plain(PlainValueStore),
    Indexed(IndexedValueStore),
}

impl ValueStore {
    pub fn new_plain(size_hint: Option<usize>) -> StoreHandle {
        Self::Plain(PlainValueStore(BaseValueStore::new(size_hint))).into()
    }

    pub fn new_indexed() -> StoreHandle {
        Self::Indexed(IndexedValueStore(BaseValueStore::new(Some(0)))).into()
    }

    pub(crate) fn finalize(&mut self, idx: ValueStoreIdx) {
        match self {
            Self::Plain(ref mut s) => s.finalize(idx),
            Self::Indexed(ref mut s) => s.finalize(idx),
        }
    }

    pub(crate) fn add_value(&mut self, data: Vec<u8>) -> usize {
        match self {
            Self::Plain(s) => s.add_value(data),
            Self::Indexed(s) => s.add_value(data),
        }
    }

    pub(crate) fn key_size(&self) -> ByteSize {
        match &self {
            Self::Plain(s) => s.key_size(),
            Self::Indexed(s) => s.key_size(),
        }
    }

    pub(crate) fn get_idx(&self) -> Option<ValueStoreIdx> {
        match &self {
            Self::Plain(s) => s.get_idx(),
            Self::Indexed(s) => s.get_idx(),
        }
    }

    pub(crate) fn kind(&self) -> ValueStoreKind {
        match &self {
            Self::Plain(_) => ValueStoreKind::Plain,
            Self::Indexed(_) => ValueStoreKind::Indexed,
        }
    }

    fn get(&self, idx: usize) -> ValueIdx {
        match self {
            Self::Plain(s) => s.get(idx),
            Self::Indexed(s) => s.get(idx),
        }
    }
}

impl WritableTell for ValueStore {
    fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        match self {
            Self::Plain(s) => s.write_data(stream),
            Self::Indexed(s) => s.write_data(stream),
        }
    }

    fn serialize_tail(&mut self, ser: &mut Serializer) -> Result<()> {
        match self {
            Self::Plain(s) => s.serialize_tail(ser),
            Self::Indexed(s) => s.serialize_tail(ser),
        }
    }
}

struct BaseValueStore {
    idx: Option<ValueStoreIdx>,
    data: Vec<(Box<[u8]>, u64)>, // The array storing the data and the index it will be written when sorted.
    sorted_indirect: Vec<usize>, // An array reindexing the data in sorted order.
    // data[sorted_indirect[i]].0 <= data[sorted_indirect[j]] for any i < j
    size: Size,
    finalized: bool,
}

impl BaseValueStore {
    pub(crate) fn new(size_hint: Option<usize>) -> Self {
        let data = Vec::with_capacity(size_hint.unwrap_or(1024));
        let sorted_indirect = Vec::with_capacity(size_hint.unwrap_or(1024));
        Self {
            idx: None,
            data,
            sorted_indirect,
            size: Size::zero(),
            finalized: false,
        }
    }

    pub(crate) fn add_value(&mut self, data: Vec<u8>) -> usize {
        // Let's act like if data is sorted when we add it
        let key = self.data.len();
        self.data.push((data.into(), 0));
        self.sorted_indirect.push(key);
        key
    }

    fn get(&self, idx: usize) -> ValueIdx {
        if !self.finalized {
            panic!("We can get only on finalized store.");
        }
        self.data[idx].1.into()
    }
}

#[repr(transparent)]
pub(crate) struct PlainValueStore(BaseValueStore);

impl PlainValueStore {
    fn size(&self) -> Size {
        self.0.size
    }

    fn finalize(&mut self, idx: ValueStoreIdx) {
        self.0.idx = Some(idx);
        self.0
            .sorted_indirect
            .par_sort_unstable_by_key(|e| &self.0.data[*e].0);
        let mut offset = 0;
        let mut last_data_key: Option<usize> = None;
        for data_key in self.0.sorted_indirect.iter_mut() {
            //let data = &mut self.0.data[*data_key];
            // If we have a last, it is the same data ?
            if let Some(i) = last_data_key {
                assert_ne!(*data_key, i);
                //let last_data = &self.0.data[i];
                if self.0.data[*data_key].0 == self.0.data[i].0 {
                    // We have a duplicate
                    self.0.data[*data_key].1 = self.0.data[i].1;
                    *data_key = i;
                    continue;
                }
            }
            // No duplicate
            let data = &mut self.0.data[*data_key];
            data.1 = offset;
            offset += data.0.len() as u64;
            last_data_key = Some(*data_key);
        }
        self.0.size = offset.into();
        self.0.finalized = true;
    }

    fn add_value(&mut self, data: Vec<u8>) -> usize {
        self.0.add_value(data)
    }

    fn get(&self, idx: usize) -> ValueIdx {
        self.0.get(idx)
    }

    fn key_size(&self) -> ByteSize {
        needed_bytes(self.size().into_u64())
    }

    fn get_idx(&self) -> Option<ValueStoreIdx> {
        self.0.idx
    }
}

impl WritableTell for PlainValueStore {
    fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        let mut serializer = Serializer::new(BlockCheck::Crc32);
        let mut last_data_key: Option<usize> = None;
        for data_key in &self.0.sorted_indirect {
            if let Some(i) = last_data_key {
                if *data_key == i {
                    // We have a duplicate
                    // skip
                    continue;
                }
            }
            last_data_key = Some(*data_key);
            serializer.write_data(&self.0.data[*data_key].0)?;
        }
        stream.write_serializer(serializer)?;
        Ok(())
    }

    fn serialize_tail(&mut self, ser: &mut Serializer) -> Result<()> {
        ser.write_u8(0x00)?;
        self.size().serialize(ser)?;
        Ok(())
    }
}

impl std::fmt::Debug for PlainValueStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlainValueStore")
            .field("idx", &self.get_idx())
            .field("size", &self.size())
            .field("key_size", &self.key_size())
            .field("data count", &self.0.data.len())
            .finish()
    }
}

#[repr(transparent)]
pub(crate) struct IndexedValueStore(BaseValueStore);
// data[sorted_indirect[i]].1 == i

impl IndexedValueStore {
    fn finalize(&mut self, idx: ValueStoreIdx) {
        self.0.idx = Some(idx);
        self.0
            .sorted_indirect
            .par_sort_by_key(|e| &self.0.data[*e].0);
        for (idx, data_key) in self.0.sorted_indirect.iter().enumerate() {
            self.0.data[*data_key].1 = idx as u64;
        }
        self.0.finalized = true;
    }

    fn add_value(&mut self, data: Vec<u8>) -> usize {
        // [TODO] This is a long search when we have a lot of values...
        let potential_idx = if self.0.data.len() >= 1024 {
            let d = data.as_slice();
            self.0
                .data
                .par_iter()
                .position_any(|(existing_data, _)| d == existing_data.as_ref())
        } else {
            self.0
                .data
                .iter()
                .position(|(existing_data, _)| data == existing_data.as_ref())
        };
        match potential_idx {
            Some(idx) => idx,
            None => {
                self.0.size += data.len();
                self.0.add_value(data)
            }
        }
    }

    fn get(&self, idx: usize) -> ValueIdx {
        self.0.get(idx)
    }

    fn key_size(&self) -> ByteSize {
        needed_bytes(self.0.sorted_indirect.len())
    }

    fn get_idx(&self) -> Option<ValueStoreIdx> {
        self.0.idx
    }
}

impl WritableTell for IndexedValueStore {
    fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        let mut serializer = Serializer::new(BlockCheck::Crc32);
        for idx in &self.0.sorted_indirect {
            serializer.write_data(&self.0.data[*idx].0)?;
        }
        stream.write_serializer(serializer)?;
        Ok(())
    }

    fn serialize_tail(&mut self, ser: &mut Serializer) -> Result<()> {
        ser.write_u8(0x01)?;
        ser.write_u64(self.0.sorted_indirect.len() as u64)?; // key count
        let data_size = self.0.size.into_u64();
        let offset_size = needed_bytes(data_size);
        offset_size.serialize(ser)?; // offset_size
        ser.write_usized(data_size, offset_size)?; // data size
        let mut offset = 0;
        if !self.0.sorted_indirect.is_empty() {
            for idx in &self.0.sorted_indirect[..(self.0.sorted_indirect.len() - 1)] {
                let data = &self.0.data[*idx].0;
                offset += data.len() as u64;
                ser.write_usized(offset, offset_size)?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for IndexedValueStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedValueStore")
            .field("idx", &self.get_idx())
            .field("size", &self.0.size)
            .field("key_size", &self.key_size())
            .field("data count", &self.0.data.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_plain() -> Result<()> {
        let creator = ValueStore::new_plain(None);
        creator.finalize(0.into());
        let buffer = vec![];
        let mut out_stream = std::io::Cursor::new(buffer);
        creator.write().unwrap().write(&mut out_stream)?;
        Ok(())
    }

    #[test]
    fn test_empty_indexed() -> Result<()> {
        let creator = ValueStore::new_indexed();
        creator.finalize(0.into());
        let buffer = vec![];
        let mut out_stream = std::io::Cursor::new(buffer);
        creator.write().unwrap().write(&mut out_stream)?;
        Ok(())
    }
}
