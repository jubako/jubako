use super::stream::*;
use super::types::*;

/// A Reader is the main trait containing stuff from "raw data".
/// A Reader may have a size.
/// It is possible to create stream from a reader.
/// Each reader is independant.
/// Data is never modified.
pub trait Reader {
    fn size(&self) -> Size;

    fn create_stream(&self, offset: Offset, end: End) -> Box<dyn Stream>;
    fn create_stream_for(&self, size_offset: SizedOffset) -> Box<dyn Stream> {
        self.create_stream(size_offset.offset, End::Size(size_offset.size))
    }
    fn create_stream_from(&self, offset: Offset) -> Box<dyn Stream> {
        self.create_stream(offset, End::None)
    }
    fn create_stream_to(&self, end: End) -> Box<dyn Stream> {
        self.create_stream(Offset(0), end)
    }
    fn create_stream_all(&self) -> Box<dyn Stream> {
        self.create_stream(Offset(0), End::None)
    }
    fn create_sub_reader(&self, offset: Offset, end: End) -> Box<dyn Reader>;

    fn read_u8(&self, offset: Offset) -> Result<u8>;
    fn read_u16(&self, offset: Offset) -> Result<u16>;
    fn read_u32(&self, offset: Offset) -> Result<u32>;
    fn read_u64(&self, offset: Offset) -> Result<u64>;
    fn read_usized(&self, offset: Offset, size: usize) -> Result<u64>;

    fn read_i8(&self, offset: Offset) -> Result<i8>;
    fn read_i16(&self, offset: Offset) -> Result<i16>;
    fn read_i32(&self, offset: Offset) -> Result<i32>;
    fn read_i64(&self, offset: Offset) -> Result<i64>;
    fn read_isized(&self, offset: Offset, size: usize) -> Result<i64>;
}
