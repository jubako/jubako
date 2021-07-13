///! All base traits use to produce structure from raw data.
use crate::bases::types::*;
use crate::primitive::*;
use std::io::Read;

/// A stream is a object streaming a reader and producing data.
/// A stream may have a size, and is positionned.
/// A stream can produce raw value, "consuming" the data and the cursor is moved.
/// Each stream is independant.
pub trait Stream: Read {
    fn tell(&self) -> Offset;
    fn size(&self) -> Size;
    fn skip(&mut self, size: Size) -> Result<()>;

    fn read_u8(&mut self) -> Result<u8> {
        let mut d = [0_u8; 1];
        self.read_exact(&mut d)?;
        Ok(read_u8(&d))
    }
    fn read_u16(&mut self) -> Result<u16> {
        let mut d = [0_u8; 2];
        self.read_exact(&mut d)?;
        Ok(read_u16(&d))
    }
    fn read_u32(&mut self) -> Result<u32> {
        let mut d = [0_u8; 4];
        self.read_exact(&mut d)?;
        Ok(read_u32(&d))
    }
    fn read_u64(&mut self) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(&mut d)?;
        Ok(read_u64(&d))
    }
    fn read_sized(&mut self, size: usize) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(&mut d[0..size])?;
        Ok(read_to_u64(size, &d))
    }
}

/// A Producable is a object that can be produce from a stream.
pub trait Producable {
    fn produce(stream: &mut dyn Stream) -> Result<Self>
    where
        Self: Sized;
}

impl Producable for Offset {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
    }
}

impl Producable for Size {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
    }
}

impl Producable for Count<u8> {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u8()?.into())
    }
}

impl Producable for Count<u16> {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u16()?.into())
    }
}

impl Producable for Count<u32> {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u32()?.into())
    }
}

impl Producable for Count<u64> {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
    }
}
