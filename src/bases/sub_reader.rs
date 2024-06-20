use super::flux::*;
use super::reader::*;
use super::types::*;
use super::{MemoryReader, Region, Source};
use std::sync::Arc;

// A wrapper around a source. Allowing access only on a region of the source
#[derive(Debug, Copy, Clone)]
pub struct SubReader<'s> {
    source: &'s Arc<dyn Source>,
    region: Region,
}

impl<'s> SubReader<'s> {
    pub fn new_from_arc(source: &'s Arc<dyn Source>, size: Size) -> Self {
        let region = Region::new_from_size(Offset::zero(), size);
        Self { source, region }
    }

    pub fn new_from_parts(source: &'s Arc<dyn Source>, region: Region) -> Self {
        Self { source, region }
    }

    pub fn to_owned(self) -> Reader {
        Reader::new_from_parts(Arc::clone(self.source), self.region)
    }

    pub fn size(&self) -> Size {
        self.region.size()
    }

    pub fn create_flux(&self, offset: Offset, size: Size) -> Flux<'s> {
        let region = self.region.cut_rel(offset, size);
        Flux::new_from_parts(self.source, region, region.begin())
    }
    pub fn create_flux_for(&self, size_offset: SizedOffset) -> Flux<'s> {
        self.create_flux(size_offset.offset, size_offset.size)
    }
    pub fn create_flux_from(&self, offset: Offset) -> Flux<'s> {
        self.create_flux(offset, self.region.size() - offset.into())
    }
    pub fn create_flux_to(&self, size: Size) -> Flux<'s> {
        self.create_flux(Offset::zero(), size)
    }
    pub fn create_flux_all(&self) -> Flux<'s> {
        self.create_flux(Offset::zero(), self.region.size())
    }
    pub fn create_sub_reader(&self, offset: Offset, size: Size) -> SubReader<'s> {
        let region = self.region.cut_rel(offset, size);
        SubReader {
            source: self.source,
            region,
        }
    }
    pub fn create_sub_memory_reader(&self, offset: Offset, size: Size) -> Result<Reader> {
        let region = self.region.cut_rel(offset, size);
        let (source, region) = Arc::clone(self.source).into_memory_source(region)?;
        Ok(Reader::new_from_parts(source.into_source(), region))
    }

    pub fn into_memory_reader(&self, offset: Offset, size: Size) -> Result<MemoryReader> {
        let region = self.region.cut_rel(offset, size);
        let (source, region) = Arc::clone(self.source).into_memory_source(region)?;
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
