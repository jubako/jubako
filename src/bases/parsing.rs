use std::{borrow::Cow, u32};

use zerocopy::{ByteOrder, LE};

use crate::{Offset, Result};

use super::ByteSize;

/// A Parser is something parsing data from a [u8]
pub trait Parser {
    fn read_slice(&mut self, size: usize) -> Result<Cow<[u8]>>;
    fn read_data(&mut self, buf: &mut [u8]) -> Result<()>;
    fn tell(&self) -> Offset;
    fn global_offset(&self) -> Offset;
    fn skip(&mut self, size: usize) -> Result<()>;

    fn read_u8(&mut self) -> Result<u8> {
        let slice = self.read_slice(1)?;
        Ok(slice[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        let slice = self.read_slice(2)?;
        Ok(LE::read_u16(&slice))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let slice = self.read_slice(4)?;
        Ok(LE::read_u32(&slice))
    }

    fn read_u64(&mut self) -> Result<u64> {
        let slice = self.read_slice(8)?;
        Ok(LE::read_u64(&slice))
    }

    fn read_usized(&mut self, size: ByteSize) -> Result<u64> {
        let size = size as usize;
        let slice = self.read_slice(size)?;
        Ok(LE::read_uint(&slice, size))
    }

    fn read_i8(&mut self) -> Result<i8> {
        let slice = self.read_slice(1)?;
        Ok(slice[0] as i8)
    }

    fn read_i16(&mut self) -> Result<i16> {
        let slice = self.read_slice(2)?;
        Ok(LE::read_i16(&slice))
    }

    fn read_i32(&mut self) -> Result<i32> {
        let slice = self.read_slice(4)?;
        Ok(LE::read_i32(&slice))
    }

    fn read_i64(&mut self) -> Result<i64> {
        let slice = self.read_slice(8)?;
        Ok(LE::read_i64(&slice))
    }

    fn read_isized(&mut self, size: ByteSize) -> Result<i64> {
        let size = size as usize;
        let slice = self.read_slice(size)?;
        Ok(LE::read_int(&slice, size))
    }
}

pub struct SliceParser<'a> {
    slice: Cow<'a, [u8]>,
    global_offset: Offset,
    offset: usize,
}

impl<'a> SliceParser<'a> {
    pub fn new(slice: Cow<'a, [u8]>, global_offset: Offset) -> Self {
        Self {
            slice,
            global_offset,
            offset: 0,
        }
    }
}

impl<'a> Parser for SliceParser<'a> {
    fn read_slice(&mut self, size: usize) -> Result<Cow<[u8]>> {
        if self.slice.len() < size + self.offset {
            return Err(format!(
                "Out of slice. {size}({}) > {}",
                self.offset,
                self.slice.len()
            )
            .into());
        }
        let slice = &self.slice[self.offset..self.offset + size];
        self.offset += size;
        Ok(Cow::Borrowed(slice))
    }

    fn read_data(&mut self, buf: &mut [u8]) -> Result<()> {
        if self.slice.len() < buf.len() + self.offset {
            return Err(format!(
                "Out of slice. {}({}) > {}",
                buf.len(),
                self.offset,
                self.slice.len()
            )
            .into());
        }
        buf.copy_from_slice(&self.slice[self.offset..self.offset + buf.len()]);
        self.offset += buf.len();
        Ok(())
    }

    fn skip(&mut self, size: usize) -> Result<()> {
        if self.slice.len() < size + self.offset {
            return Err(format!(
                "Out of slice. {size}({}) > {}",
                self.offset,
                self.slice.len()
            )
            .into());
        }
        self.offset += size;
        Ok(())
    }

    fn tell(&self) -> Offset {
        self.offset.into()
    }

    fn global_offset(&self) -> Offset {
        self.global_offset + self.offset
    }
}

/// A Producable is a object that can be produce from a parser.
pub trait Parsable {
    type Output;
    fn parse(parser: &mut impl Parser) -> Result<Self::Output>
    where
        Self::Output: Sized;
}

pub trait SizedParsable: Parsable {
    const SIZE: usize;
}

impl Parsable for u8 {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        parser.read_u8()
    }
}
impl Parsable for u16 {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        parser.read_u16()
    }
}
impl Parsable for u32 {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        parser.read_u32()
    }
}
impl Parsable for u64 {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        parser.read_u64()
    }
}

impl SizedParsable for u8 {
    const SIZE: usize = 1;
}
impl SizedParsable for u16 {
    const SIZE: usize = 2;
}
impl SizedParsable for u32 {
    const SIZE: usize = 4;
}
impl SizedParsable for u64 {
    const SIZE: usize = 8;
}
