use super::flux::*;
use super::sub_reader::*;
use super::types::*;
use super::{MemoryReader, Region, Source};
use std::sync::Arc;

// A wrapper around a source. Allowing access only on a region of the source
#[derive(Debug, Clone)]
pub struct Reader {
    source: Arc<dyn Source>,
    region: Region,
}

impl Reader {
    pub fn new<T: Source + 'static>(source: T, end: End) -> Self {
        Self::new_from_arc(Arc::new(source), end)
    }

    pub fn new_from_parts(source: Arc<dyn Source>, region: Region) -> Self {
        Self { source, region }
    }

    pub fn new_from_arc(source: Arc<dyn Source>, end: End) -> Self {
        let region = Region::new_to_end(Offset::zero(), end, source.size());
        Self { source, region }
    }

    pub fn size(&self) -> Size {
        self.region.size()
    }

    pub fn create_flux(&self, offset: Offset, end: End) -> Flux {
        let region = self.region.cut_rel(offset, end);
        Flux::new_from_parts(&self.source, region, region.begin())
    }
    pub fn create_flux_for(&self, size_offset: SizedOffset) -> Flux {
        self.create_flux(size_offset.offset, End::Size(size_offset.size))
    }
    pub fn create_flux_from(&self, offset: Offset) -> Flux {
        self.create_flux(offset, End::None)
    }
    pub fn create_flux_to(&self, end: End) -> Flux {
        self.create_flux(Offset::zero(), end)
    }
    pub fn create_flux_all(&self) -> Flux {
        self.create_flux(Offset::zero(), End::None)
    }

    pub fn as_sub_reader(&self) -> SubReader {
        self.create_sub_reader(Offset::zero(), End::None)
    }

    pub fn create_sub_reader(&self, offset: Offset, end: End) -> SubReader {
        let region = self.region.cut_rel(offset, end);
        SubReader::new_from_parts(&self.source, region)
    }
    pub fn create_sub_memory_reader(&self, offset: Offset, end: End) -> Result<Reader> {
        let region = self.region.cut_rel(offset, end);
        let (source, region) = Arc::clone(&self.source).into_memory(region)?;
        Ok(Reader { source, region })
    }

    pub fn into_memory_reader(&self, offset: Offset, end: End) -> Result<MemoryReader> {
        let region = self.region.cut_rel(offset, end);
        let (source, region) = Arc::clone(&self.source).into_memory_source(region)?;
        Ok(MemoryReader::new_from_parts(source, region))
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

    pub fn read_i8(&self, offset: Offset) -> Result<i8> {
        self.source.read_i8(self.region.begin() + offset)
    }
    pub fn read_i16(&self, offset: Offset) -> Result<i16> {
        self.source.read_i16(self.region.begin() + offset)
    }
    pub fn read_i32(&self, offset: Offset) -> Result<i32> {
        self.source.read_i32(self.region.begin() + offset)
    }
    pub fn read_i64(&self, offset: Offset) -> Result<i64> {
        self.source.read_i64(self.region.begin() + offset)
    }
    pub fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        self.source.read_isized(self.region.begin() + offset, size)
    }
}

impl From<SubReader<'_>> for Reader {
    fn from(sub: SubReader) -> Self {
        sub.to_owned()
    }
}

impl<T> From<T> for Reader
where
    T: Source + 'static,
{
    fn from(source: T) -> Self {
        Self::new(source, End::None)
    }
}
