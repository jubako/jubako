use crate::bases::*;
use std::fmt::Debug;
pub use std::io::Result as IoResult;
use std::io::{Read, Seek, Write};
use zerocopy::byteorder::little_endian::{U16, U32, U64};
use zerocopy::{AsBytes, ByteOrder, LittleEndian as LE};

/// A OutStream is a object on which we can write data.
pub trait OutStream: Write + Seek + Send + Debug {
    fn copy(&mut self, reader: Box<dyn crate::creator::InputReader>) -> IoResult<u64>;

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

pub trait InOutStream: OutStream + Read {}

/// A Writable is a object we can write on a `Write` trait.
pub trait Writable {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize>;
}

impl OutStream for std::fs::File {
    fn copy(&mut self, reader: Box<dyn crate::creator::InputReader>) -> IoResult<u64> {
        match reader.get_file_source() {
            crate::creator::MaybeFileReader::Yes(mut input_file) => {
                std::io::copy(&mut input_file, self)
            }
            crate::creator::MaybeFileReader::No(mut reader) => std::io::copy(reader.as_mut(), self),
        }
    }
}

impl<T> OutStream for std::io::Cursor<T>
where
    std::io::Cursor<T>: Write + Seek + Send + std::fmt::Debug,
{
    fn copy(&mut self, reader: Box<dyn crate::creator::InputReader>) -> IoResult<u64> {
        match reader.get_file_source() {
            crate::creator::MaybeFileReader::Yes(mut input_file) => {
                std::io::copy(&mut input_file, self)
            }
            crate::creator::MaybeFileReader::No(mut reader) => std::io::copy(reader.as_mut(), self),
        }
    }
}

impl<T> OutStream for std::io::BufWriter<T>
where
    T: Write + Seek + Send + Debug,
{
    fn copy(&mut self, reader: Box<dyn crate::creator::InputReader>) -> IoResult<u64> {
        match reader.get_file_source() {
            crate::creator::MaybeFileReader::Yes(mut input_file) => {
                std::io::copy(&mut input_file, self)
            }
            crate::creator::MaybeFileReader::No(mut reader) => std::io::copy(reader.as_mut(), self),
        }
    }
}

impl<O> OutStream for Skip<O>
where
    O: OutStream,
{
    fn copy(&mut self, reader: Box<dyn crate::creator::InputReader>) -> IoResult<u64> {
        self.inner_mut().copy(reader)
    }
}

impl<T> InOutStream for T where T: OutStream + Read {}
