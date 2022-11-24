use super::WritableTell;
use crate::bases::*;

#[derive(Debug)]
pub struct PlainValueStore {
    idx: ValueStoreIdx,
    data: Vec<Vec<u8>>,
    size: Size,
}

impl PlainValueStore {
    pub fn new(idx: ValueStoreIdx) -> Self {
        Self {
            idx,
            data: vec![],
            size: Size::zero(),
        }
    }

    pub fn add_value(&mut self, data: &[u8]) -> u64 {
        let offset = self.size.into_u64();
        self.data.push(data.to_vec());
        self.size += 1 + data.len();
        offset
    }

    pub fn key_size(&self) -> u16 {
        needed_bytes(self.size.into_usize()) as u16
    }

    pub fn get_idx(&self) -> ValueStoreIdx {
        self.idx
    }
}

impl WritableTell for PlainValueStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for data in &self.data {
            PString::write_string(data, stream)?;
        }
        Ok(())
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        self.size.write(stream)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct IndexedValueStore {
    idx: ValueStoreIdx,
    data: Vec<Vec<u8>>,
    entries_offset: Vec<usize>,
}

impl IndexedValueStore {
    pub fn new(idx: ValueStoreIdx) -> Self {
        Self {
            idx,
            data: vec![],
            entries_offset: vec![],
        }
    }

    fn current_offset(&self) -> usize {
        self.entries_offset.last().copied().unwrap_or(0)
    }

    pub fn add_value(&mut self, data: &[u8]) -> u64 {
        self.data.push(data.to_vec());
        self.entries_offset.push(self.current_offset() + data.len());
        self.entries_offset.len() as u64 - 1
    }

    pub fn key_size(&self) -> u16 {
        needed_bytes(self.entries_offset.len()) as u16
    }

    pub fn get_idx(&self) -> ValueStoreIdx {
        self.idx
    }
}

impl WritableTell for IndexedValueStore {
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
pub enum ValueStoreKind {
    Plain,
    Indexed,
}

#[derive(Debug)]
pub enum ValueStore {
    PlainValueStore(PlainValueStore),
    IndexedValueStore(IndexedValueStore),
}

impl ValueStore {
    pub fn new(kind: ValueStoreKind, idx: ValueStoreIdx) -> ValueStore {
        match kind {
            ValueStoreKind::Plain => ValueStore::PlainValueStore(PlainValueStore::new(idx)),
            ValueStoreKind::Indexed => ValueStore::IndexedValueStore(IndexedValueStore::new(idx)),
        }
    }

    pub fn add_value(&mut self, data: &[u8]) -> u64 {
        match self {
            ValueStore::PlainValueStore(s) => s.add_value(data),
            ValueStore::IndexedValueStore(s) => s.add_value(data),
        }
    }

    pub(crate) fn key_size(&self) -> u16 {
        match &self {
            ValueStore::PlainValueStore(s) => s.key_size(),
            ValueStore::IndexedValueStore(s) => s.key_size(),
        }
    }

    pub(crate) fn get_idx(&self) -> ValueStoreIdx {
        match &self {
            ValueStore::PlainValueStore(s) => s.get_idx(),
            ValueStore::IndexedValueStore(s) => s.get_idx(),
        }
    }
}

impl WritableTell for ValueStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        match &self {
            ValueStore::PlainValueStore(s) => s.write_data(stream),
            ValueStore::IndexedValueStore(s) => s.write_data(stream),
        }
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        match &self {
            ValueStore::PlainValueStore(s) => {
                stream.write_u8(0x00)?;
                s.write_tail(stream)
            }
            ValueStore::IndexedValueStore(s) => {
                stream.write_u8(0x01)?;
                s.write_tail(stream)
            }
        }
    }
}
