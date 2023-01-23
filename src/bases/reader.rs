use super::primitive::*;
use super::stream::*;
use super::types::*;
use super::Source;
use std::rc::Rc;

// A wrapper arount someting to implement Reader trait
#[derive(Debug)]
pub struct Reader {
    source: Rc<dyn Source>,
    origin: Offset,
    end: Offset,
}

impl Reader {
    pub fn new<T: Source + 'static>(source: T, end: End) -> Self {
        Reader::new_from_rc(Rc::new(source), end)
    }

    pub fn new_from_rc<T: Source + 'static>(source: Rc<T>, end: End) -> Self {
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

    pub fn create_stream(&self, offset: Offset, end: End) -> Stream {
        let source = Rc::clone(&self.source);
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        assert!(end <= self.end);
        Stream::new_from_parts(source, origin, end, origin)
    }
    pub fn create_stream_for(&self, size_offset: SizedOffset) -> Stream {
        self.create_stream(size_offset.offset, End::Size(size_offset.size))
    }
    pub fn create_stream_from(&self, offset: Offset) -> Stream {
        self.create_stream(offset, End::None)
    }
    pub fn create_stream_to(&self, end: End) -> Stream {
        self.create_stream(Offset::zero(), end)
    }
    pub fn create_stream_all(&self) -> Stream {
        self.create_stream(Offset::zero(), End::None)
    }
    pub fn create_sub_reader(&self, offset: Offset, end: End) -> Reader {
        let source = Rc::clone(&self.source);
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        assert!(end <= self.end);
        Reader {
            source,
            origin,
            end,
        }
    }
    pub fn create_sub_memory_reader(&self, offset: Offset, end: End) -> Result<Reader> {
        let origin = self.origin + offset;
        let size = match end {
            End::None => self.end - origin,
            End::Offset(o) => o - offset,
            End::Size(s) => s,
        };
        let (source, origin, end) =
            Rc::clone(&self.source).into_memory(origin, size.into_usize())?;
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
        let slice = self.source.slice_1(self.origin + offset)?;
        Ok(read_u8(&slice))
    }
    pub fn read_u16(&self, offset: Offset) -> Result<u16> {
        let slice = self.source.slice_2(self.origin + offset)?;
        Ok(read_u16(&slice))
    }
    pub fn read_u32(&self, offset: Offset) -> Result<u32> {
        let slice = self.source.slice_4(self.origin + offset)?;
        Ok(read_u32(&slice))
    }
    pub fn read_u64(&self, offset: Offset) -> Result<u64> {
        let slice = self.source.slice_8(self.origin + offset)?;
        Ok(read_u64(&slice))
    }
    pub fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        let slice = self.source.slice_sized(self.origin + offset, size)?;
        let size = size as usize;
        Ok(read_to_u64(size, &slice[..size]))
    }

    pub fn read_i8(&self, offset: Offset) -> Result<i8> {
        let slice = self.source.slice_1(self.origin + offset)?;
        Ok(read_i8(&slice))
    }
    pub fn read_i16(&self, offset: Offset) -> Result<i16> {
        let slice = self.source.slice_2(self.origin + offset)?;
        Ok(read_i16(&slice))
    }
    pub fn read_i32(&self, offset: Offset) -> Result<i32> {
        let slice = self.source.slice_4(self.origin + offset)?;
        Ok(read_i32(&slice))
    }
    pub fn read_i64(&self, offset: Offset) -> Result<i64> {
        let slice = self.source.slice_8(self.origin + offset)?;
        Ok(read_i64(&slice))
    }
    pub fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        let slice = self.source.slice_sized(self.origin + offset, size)?;
        let size = size as usize;
        Ok(read_to_i64(size, &slice[..size]))
    }
}
