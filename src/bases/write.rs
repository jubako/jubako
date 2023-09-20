use crate::bases::*;
pub use std::io::Result as IoResult;
use std::io::{Seek, Write};
use zerocopy::byteorder::little_endian::{U16, U32, U64};
use zerocopy::{AsBytes, ByteOrder, LittleEndian as LE};

/// A OutStream is a object on which we can write data.
pub trait OutStream: Write + Seek {
    fn tell(&mut self) -> Offset {
        self.stream_position().unwrap().into()
    }

    fn write_u8(&mut self, value: u8) -> IoResult<usize> {
        self.write_all(value.as_bytes())?;
        Ok(1)
    }
    fn write_u16(&mut self, value: u16) -> IoResult<usize> {
        let d = U16::from(value);
        self.write_all(d.as_bytes())?;
        Ok(2)
    }
    fn write_u32(&mut self, value: u32) -> IoResult<usize> {
        let d = U32::from(value);
        self.write_all(d.as_bytes())?;
        Ok(4)
    }
    fn write_u64(&mut self, value: u64) -> IoResult<usize> {
        let d = U64::from(value);
        self.write_all(d.as_bytes())?;
        Ok(8)
    }
    fn write_usized(&mut self, value: u64, size: ByteSize) -> IoResult<usize> {
        let d = U64::from(value);
        let size = size as usize;
        self.write_all(&d.as_bytes()[..size])?;
        Ok(size)
    }
    fn write_isized(&mut self, value: i64, size: ByteSize) -> IoResult<usize> {
        let mut d = [0_u8; 8];
        let size = size as usize;
        LE::write_int(&mut d, value, size);
        self.write_all(&d[..size])?;
        Ok(size)
    }
    fn write_data(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }
}

/// A Writable is a object we can write on a `Write` trait.
pub trait Writable {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize>;
}

impl<T> OutStream for T where T: Write + Seek {}
