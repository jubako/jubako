use super::flux::*;
use super::sub_reader::*;
use super::types::*;
use super::{MemoryReader, Source};
use std::sync::Arc;

// A wrapper around a source. Allowing access only on a region of the source
#[derive(Debug)]
pub struct Reader {
    source: Arc<dyn Source>,
    origin: Offset,
    end: Offset,
}

impl Reader {
    pub fn new<T: Source + 'static>(source: T, end: End) -> Self {
        Self::new_from_arc(Arc::new(source), end)
    }

    pub fn new_from_parts(source: Arc<dyn Source>, origin: Offset, end: Offset) -> Self {
        Self {
            source,
            origin,
            end,
        }
    }

    pub fn new_from_arc(source: Arc<dyn Source>, end: End) -> Self {
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

    pub fn create_flux(&self, offset: Offset, end: End) -> Flux {
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        debug_assert!(end <= self.end);
        Flux::new_from_parts(&self.source, origin, end, origin)
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
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        debug_assert!(end <= self.end);
        SubReader::new_from_parts(&self.source, origin, end)
    }
    pub fn create_sub_memory_reader(&self, offset: Offset, end: End) -> Result<Reader> {
        let origin = self.origin + offset;
        let size = match end {
            End::None => self.end - origin,
            End::Offset(o) => o - offset,
            End::Size(s) => s,
        };
        let (source, origin, end) =
            Arc::clone(&self.source).into_memory(origin, size.into_usize())?;
        let end = match end {
            End::None => source.size().into(),
            End::Offset(o) => origin + o,
            End::Size(s) => origin + s,
        };
        Ok(Reader {
            source,
            origin,
            end,
        })
    }

    pub fn into_memory_reader(&self) -> Result<MemoryReader> {
        let size = self.end - self.origin;
        let (source, origin, end) =
            Arc::clone(&self.source).into_memory_source(self.origin, size.into_usize())?;
        let end = match end {
            End::None => source.size().into(),
            End::Offset(o) => origin + o,
            End::Size(s) => origin + s,
        };
        Ok(MemoryReader::new_from_parts(source, origin, end))
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

    pub fn read_i8(&self, offset: Offset) -> Result<i8> {
        self.source.read_i8(self.origin + offset)
    }
    pub fn read_i16(&self, offset: Offset) -> Result<i16> {
        self.source.read_i16(self.origin + offset)
    }
    pub fn read_i32(&self, offset: Offset) -> Result<i32> {
        self.source.read_i32(self.origin + offset)
    }
    pub fn read_i64(&self, offset: Offset) -> Result<i64> {
        self.source.read_i64(self.origin + offset)
    }
    pub fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        self.source.read_isized(self.origin + offset, size)
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
