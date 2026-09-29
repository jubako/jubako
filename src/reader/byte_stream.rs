use crate::bases::*;
use std::io::Read;

/// A `Read` struct on top of bytes contained in Jubako
///
/// A `ByteStream` allow to read from a [ByteRegion].
pub struct ByteStream(Box<dyn ReadSized>);

impl ByteStream {
    pub(crate) fn new(read: Box<dyn ReadSized>) -> Self {
        Self(read)
    }

    /// The size of the data left to be read
    pub fn size_left(&self) -> u64 {
        self.0.size_left().into_u64()
    }

    /// The full size of the ByteStream
    pub fn size(&self) -> u64 {
        self.0.size().into_u64()
    }

    /// The current offset in the ByteStream
    pub fn offset(&self) -> u64 {
        self.0.offset().into_u64()
    }
}

impl Read for ByteStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}
