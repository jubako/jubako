use super::WritableTell;
use crate::bases::*;

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
