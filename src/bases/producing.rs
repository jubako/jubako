///! All base traits use to produce structure from raw data.
use crate::bases::types::*;
use crate::primitive::*;
use std::io::{Read, Seek};

/// A producer is the main trait producing stuff from "raw data".
/// A producer may have a size, and is positionned.
/// The cursor can be move.
/// Producing a value "consumes" the data and the cursor is moved.
/// It is possible to create subproducer, a producer reading the sub range of tha data.
/// Each producer are independant.
/// Data is never modified.
pub trait Producer: Read + Seek {
    fn tell_cursor(&self) -> Offset;
    fn size(&self) -> Size;

    fn sub_producer_at(&self, offset: Offset, end: End) -> Box<dyn Producer>;

    fn read_u8(&mut self) -> Result<u8> {
        let mut d = [0_u8; 1];
        self.read_exact(&mut d)?;
        let v = read_u8(&d);
        Ok(v)
    }
    fn read_u16(&mut self) -> Result<u16> {
        let mut d = [0_u8; 2];
        self.read_exact(&mut d)?;
        let v = read_u16(&d);
        Ok(v)
    }
    fn read_u32(&mut self) -> Result<u32> {
        let mut d = [0_u8; 4];
        self.read_exact(&mut d)?;
        let v = read_u32(&d);
        Ok(v)
    }
    fn read_u64(&mut self) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(&mut d)?;
        let v = read_u64(&d);
        Ok(v)
    }
    fn read_sized(&mut self, size: usize) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(&mut d[0..size])?;
        let v = read_to_u64(size, &d);
        Ok(v)
    }
}

pub trait Producable {
    fn produce(producer: &mut dyn Producer) -> Result<Self>
    where
        Self: Sized;
}

impl Producable for Offset {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(producer.read_u64()?.into())
    }
}

impl Producable for Size {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(producer.read_u64()?.into())
    }
}

impl Producable for Count<u16> {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(producer.read_u16()?.into())
    }
}

impl Producable for Count<u32> {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(producer.read_u32()?.into())
    }
}
