use super::types::*;
use super::{MemorySource, Reader, Region};
use std::sync::Arc;

// A wrapper around a source. Allowing access only on a region of the source
#[derive(Debug)]
pub struct MemoryReader {
    source: Arc<dyn MemorySource>,
    region: Region,
}

impl MemoryReader {
    pub fn new<T: MemorySource + 'static>(source: T, end: End) -> Self {
        Self::new_from_arc(Arc::new(source), end)
    }

    pub fn new_from_parts(source: Arc<dyn MemorySource>, region: Region) -> Self {
        Self { source, region }
    }

    pub fn new_from_arc(source: Arc<dyn MemorySource>, end: End) -> Self {
        let region = Region::new_to_end(Offset::zero(), end, source.size());
        Self { source, region }
    }

    pub fn size(&self) -> Size {
        self.region.size()
    }

    /// Get a slice from the reader.
    /// This is usefull only if this is a memory reader, panic if not
    /// [TODO] Use a new trait/type for this.
    pub fn get_slice(&self, offset: Offset, end: End) -> Result<&[u8]> {
        let region = self.region.cut_rel(offset, end);
        // We know for sure that our reader in inside the region of our source.
        // This is also true for SeekableDecoder as it has already called
        // decode_to before casting itself to a MemorySource
        unsafe { self.source.get_slice_unchecked(region) }
    }

    pub fn read_u8(&self, offset: Offset) -> Result<u8> {
        self.source.read_u8(self.region.begin() + offset)
    }
    pub fn read_u16(&self, offset: Offset) -> Result<u16> {
        self.source.read_u16(self.region.begin() + offset)
    }
    pub fn read_u32(&self, offset: Offset) -> Result<u32> {
        self.source.read_u32(self.region.begin() + offset)
    }
    pub fn read_u64(&self, offset: Offset) -> Result<u64> {
        self.source.read_u64(self.region.begin() + offset)
    }
    pub fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        self.source.read_usized(self.region.begin() + offset, size)
    }
}

impl TryFrom<Reader> for MemoryReader {
    type Error = Error;
    fn try_from(reader: Reader) -> Result<Self> {
        reader.into_memory_reader(Offset::zero(), End::None)
    }
}
