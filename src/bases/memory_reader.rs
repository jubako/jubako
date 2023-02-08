use super::types::*;
use super::{MemorySource, Reader};
use std::sync::Arc;

// A wrapper around a source. Allowing access only on a region of the source
#[derive(Debug)]
pub struct MemoryReader {
    source: Arc<dyn MemorySource>,
    origin: Offset,
    end: Offset,
}

impl MemoryReader {
    pub fn new<T: MemorySource + 'static>(source: T, end: End) -> Self {
        Self::new_from_arc(Arc::new(source), end)
    }

    pub fn new_from_parts(source: Arc<dyn MemorySource>, origin: Offset, end: Offset) -> Self {
        Self {
            source,
            origin,
            end,
        }
    }

    pub fn new_from_arc(source: Arc<dyn MemorySource>, end: End) -> Self {
        let end = match end {
            End::None => source.size().into(),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        Self {
            source,
            origin: Offset::zero(),
            end,
        }
    }

    pub fn size(&self) -> Size {
        self.end - self.origin
    }

    /// Get a slice from the reader.
    /// This is usefull only if this is a memory reader, panic if not
    /// [TODO] Use a new trait/type for this.
    pub fn get_slice(&self, offset: Offset, end: End) -> Result<&[u8]> {
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        self.source.get_slice(origin, end)
    }

    pub fn read_u8(&self, offset: Offset) -> Result<u8> {
        self.source.read_u8(self.origin + offset)
    }
    pub fn read_u16(&self, offset: Offset) -> Result<u16> {
        self.source.read_u16(self.origin + offset)
    }
    pub fn read_u32(&self, offset: Offset) -> Result<u32> {
        self.source.read_u32(self.origin + offset)
    }
    pub fn read_u64(&self, offset: Offset) -> Result<u64> {
        self.source.read_u64(self.origin + offset)
    }
    pub fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        self.source.read_usized(self.origin + offset, size)
    }
}

impl TryFrom<Reader> for MemoryReader {
    type Error = Error;
    fn try_from(reader: Reader) -> Result<Self> {
        reader.into_memory_reader()
    }
}
