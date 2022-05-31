use crate::bases::*;
use primitive::*;
use std::fs::File;
pub use std::io::Result as IoResult;
use std::io::{Seek, Write};

/// A OutStream is a object on which we can write data.
pub trait OutStream: Write + Seek {
    fn tell(&mut self) -> Offset;

    fn write_u8(&mut self, value: u8) -> IoResult<()> {
        let mut d = [0_u8; 1];
        write_u8(value, &mut d);
        self.write_all(&d)
    }
    fn write_u16(&mut self, value: u16) -> IoResult<()> {
        let mut d = [0_u8; 2];
        write_u16(value, &mut d);
        self.write_all(&d)
    }
    fn write_u32(&mut self, value: u32) -> IoResult<()> {
        let mut d = [0_u8; 4];
        write_u32(value, &mut d);
        self.write_all(&d)
    }
    fn write_u64(&mut self, value: u64) -> IoResult<()> {
        let mut d = [0_u8; 8];
        write_u64(value, &mut d);
        self.write_all(&d)
    }
    fn write_sized(&mut self, value: u64, size: usize) -> IoResult<()> {
        let mut d = [0_u8; 8];
        write_from_u64(value, size, &mut d);
        self.write_all(&d[..size])
    }
}

/// A Writable is a object we can write on a `Write` trait.
pub trait Writable {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()>;
}

impl OutStream for File {
    fn tell(&mut self) -> Offset {
        self.stream_position().unwrap().into()
    }
}
