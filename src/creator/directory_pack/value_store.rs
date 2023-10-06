use crate::bases::*;
use crate::creator::private::WritableTell;
use rayon::prelude::*;

use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct StoreHandle(Arc<RwLock<ValueStore>>);

impl StoreHandle {
    pub fn key_size(&self) -> ByteSize {
        self.0.read().unwrap().key_size()
    }

    pub fn get_idx(&self) -> Option<ValueStoreIdx> {
        self.0.read().unwrap().get_idx()
    }

    pub fn add_value(&self, data: &[u8]) -> Bound<u64> {
        self.0.write().unwrap().add_value(data)
    }

    pub fn finalize(&self, idx: ValueStoreIdx) {
        self.0.write().unwrap().finalize(idx)
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
    pub fn new_plain() -> StoreHandle {
        Self::Plain(PlainValueStore(BaseValueStore::new())).into()
    }

    pub fn new_indexed() -> StoreHandle {
        Self::Indexed(IndexedValueStore(BaseValueStore::new())).into()
    }

    pub fn finalize(&mut self, idx: ValueStoreIdx) {
        match self {
            Self::Plain(ref mut s) => s.finalize(idx),
            Self::Indexed(ref mut s) => s.finalize(idx),
        }
    }

    pub fn add_value(&mut self, data: &[u8]) -> Bound<u64> {
        match self {
            Self::Plain(s) => s.add_value(data),
            Self::Indexed(s) => s.add_value(data),
        }
    }

    pub fn key_size(&self) -> ByteSize {
        match &self {
            Self::Plain(s) => s.key_size(),
            Self::Indexed(s) => s.key_size(),
        }
    }

    pub fn get_idx(&self) -> Option<ValueStoreIdx> {
        match &self {
            Self::Plain(s) => s.get_idx(),
            Self::Indexed(s) => s.get_idx(),
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

    fn write_tail(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        match self {
            Self::Plain(s) => s.write_tail(stream),
            Self::Indexed(s) => s.write_tail(stream),
        }
    }
}

pub struct BaseValueStore {
    idx: Option<ValueStoreIdx>,
    data: Vec<Vec<u8>>,
    sorted_indirect: Vec<(usize, Vow<u64>)>,
    size: Size,
}

impl BaseValueStore {
    pub fn new() -> Self {
        Self {
            idx: None,
            data: vec![],
            sorted_indirect: vec![],
            size: Size::zero(),
        }
    }

    pub fn add_value(&mut self, data: &[u8]) -> Bound<u64> {
        self.data.push(data.to_vec());
        let vow = Vow::new(0);
        let bound = vow.bind();
        self.sorted_indirect.push((self.data.len() - 1, vow));
        bound
    }
}

#[repr(transparent)]
pub struct PlainValueStore(BaseValueStore);

impl PlainValueStore {
    fn size(&self) -> Size {
        self.0.size
    }

    fn finalize(&mut self, idx: ValueStoreIdx) {
        self.0.idx = Some(idx);
        self.0
            .sorted_indirect
            .par_sort_unstable_by_key(|e| &self.0.data[e.0]);
        let mut offset = 0;
        let mut last_data_idx: Option<usize> = None;
        for (idx, vow) in self.0.sorted_indirect.iter_mut() {
            let data = &self.0.data[*idx];
            if let Some(i) = last_data_idx {
                if data == &self.0.data[i] {
                    // We have a duplicate
                    *idx = i;
                    vow.fulfil(offset - (data.len() as u64));
                    continue;
                }
            }
            // No duplicate
            vow.fulfil(offset);
            offset += data.len() as u64;
            last_data_idx = Some(*idx);
        }
        self.0.size = offset.into();
    }

    fn add_value(&mut self, data: &[u8]) -> Bound<u64> {
        self.0.add_value(data)
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
        let mut last_data_idx: Option<usize> = None;
        for (idx, _) in &self.0.sorted_indirect {
            if let Some(i) = last_data_idx {
                if *idx == i {
                    // We have a duplicate
                    // skip
                    continue;
                }
            }
            last_data_idx = Some(*idx);
            stream.write_data(&self.0.data[*idx])?;
        }
        Ok(())
    }

    fn write_tail(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        stream.write_u8(0x00)?;
        self.size().write(stream)?;
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
pub struct IndexedValueStore(BaseValueStore);

impl IndexedValueStore {
    fn finalize(&mut self, idx: ValueStoreIdx) {
        self.0.idx = Some(idx);
        self.0
            .sorted_indirect
            .par_sort_by_key(|e| &self.0.data[e.0]);
        for (idx, (_, vow)) in self.0.sorted_indirect.iter().enumerate() {
            vow.fulfil(idx as u64);
        }
    }

    fn add_value(&mut self, data: &[u8]) -> Bound<u64> {
        for (idx, vow) in self.0.sorted_indirect.iter() {
            let existing_data = &self.0.data[*idx];
            if data == existing_data.as_slice() {
                // We have found a duplicate
                return vow.bind();
            }
        }
        self.0.size += data.len();
        self.0.add_value(data)
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
        for (idx, _) in &self.0.sorted_indirect {
            stream.write_data(&self.0.data[*idx])?;
        }
        Ok(())
    }

    fn write_tail(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        stream.write_u8(0x01)?;
        stream.write_u64(self.0.sorted_indirect.len() as u64)?; // key count
        let data_size = self.0.size.into_u64();
        let offset_size = needed_bytes(data_size);
        offset_size.write(stream)?; // offset_size
        stream.write_usized(data_size, offset_size)?; // data size
        let mut offset = 0;
        for (idx, _) in &self.0.sorted_indirect[..(self.0.sorted_indirect.len() - 1)] {
            let data = &self.0.data[*idx];
            offset += data.len() as u64;
            stream.write_usized(offset, offset_size)?;
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
