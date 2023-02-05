use super::private::WritableTell;
use crate::bases::*;

#[derive(Debug)]
pub struct BaseValueStore {
    idx: ValueStoreIdx,
    data: Vec<Vec<u8>>,
    sorted_indirect: Vec<(usize, Vow<u64>)>,
    size: Size,
}

impl BaseValueStore {
    pub fn new(idx: ValueStoreIdx) -> Self {
        Self {
            idx,
            data: vec![],
            sorted_indirect: vec![],
            size: Size::zero(),
        }
    }

    fn get_bound_or_insert(&self, data: &[u8]) -> std::result::Result<Bound<u64>, usize> {
        match self
            .sorted_indirect
            .binary_search_by(|entry| self.data[entry.0].as_slice().cmp(data))
        {
            Ok(sorted_idx) => Ok(self.sorted_indirect[sorted_idx].1.bind()),
            Err(insertion_idx) => Err(insertion_idx),
        }
    }

    pub fn add_value<F: FnOnce(&mut Self, usize)>(
        &mut self,
        data: &[u8],
        fix_offset: F,
    ) -> Bound<u64> {
        match self.get_bound_or_insert(data) {
            Ok(bound) => bound,
            Err(insertion_idx) => {
                self.data.push(data.to_vec());
                self.size += data.len();
                let vow = Vow::new(0);
                let bound = vow.bind();
                self.sorted_indirect
                    .insert(insertion_idx, (self.data.len() - 1, vow));
                fix_offset(self, insertion_idx);
                bound
            }
        }
    }
}

#[derive(Debug)]
pub struct PlainValueStore(BaseValueStore);

impl PlainValueStore {
    pub fn new(idx: ValueStoreIdx) -> Self {
        Self(BaseValueStore::new(idx))
    }

    fn fix_offset(s: &mut BaseValueStore, starting_point: usize) {
        if starting_point == s.sorted_indirect.len() - 1 {
            // We are at the end of the array
            if starting_point != 0 {
                // We are not at the begining
                let (idx, vow) = &s.sorted_indirect[starting_point - 1];
                let offset = vow.get() + 1 + s.data[*idx].len() as u64;
                s.sorted_indirect[starting_point].1.fulfil(offset);
            }
            // If we are at end and beggining, we have only one element, nothing to do
        } else {
            // We are not at the end.
            // The one following us contains the offset to start with
            let mut offset = s.sorted_indirect[starting_point + 1].1.get();
            for (idx, vow) in s.sorted_indirect.iter().skip(starting_point) {
                vow.fulfil(offset);
                let data = &s.data[*idx];
                offset += 1 + data.len() as u64;
            }
        }
    }

    pub fn add_value(&mut self, data: &[u8]) -> Bound<u64> {
        self.0.add_value(data, Self::fix_offset)
    }

    pub fn size(&self) -> Size {
        self.0.size + Size::from(self.0.data.len())
    }

    pub fn key_size(&self) -> u16 {
        needed_bytes(self.size().into_u64()) as u16
    }

    pub fn get_idx(&self) -> ValueStoreIdx {
        self.0.idx
    }
}

impl WritableTell for PlainValueStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for (idx, _) in &self.0.sorted_indirect {
            PString::write_string(&self.0.data[*idx], stream)?;
        }
        Ok(())
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        self.size().write(stream)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct IndexedValueStore(BaseValueStore);

impl IndexedValueStore {
    pub fn new(idx: ValueStoreIdx) -> Self {
        Self(BaseValueStore::new(idx))
    }

    fn fix_offset(s: &mut BaseValueStore, starting_point: usize) {
        for (idx, (_, vow)) in s.sorted_indirect.iter().enumerate().skip(starting_point) {
            vow.fulfil(idx as u64);
        }
    }

    pub fn add_value(&mut self, data: &[u8]) -> Bound<u64> {
        self.0.add_value(data, Self::fix_offset)
    }

    pub fn key_size(&self) -> u16 {
        needed_bytes(self.0.sorted_indirect.len()) as u16
    }

    pub fn get_idx(&self) -> ValueStoreIdx {
        self.0.idx
    }
}

impl WritableTell for IndexedValueStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for (idx, _) in &self.0.sorted_indirect {
            stream.write_data(&self.0.data[*idx])?;
        }
        Ok(())
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        stream.write_u64(self.0.sorted_indirect.len() as u64)?; // key count
        let data_size = self.0.size.into_u64();
        let offset_size = needed_bytes(data_size);
        stream.write_u8(offset_size as u8)?; // offset_size
        stream.write_sized(data_size, offset_size)?; // data size
        let mut offset = 0;
        for (idx, _) in &self.0.sorted_indirect[..(self.0.sorted_indirect.len() - 1)] {
            let data = &self.0.data[*idx];
            offset += data.len() as u64;
            stream.write_sized(offset, offset_size)?;
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

    pub fn add_value(&mut self, data: &[u8]) -> Bound<u64> {
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
