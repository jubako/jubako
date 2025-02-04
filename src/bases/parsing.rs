use std::borrow::Cow;

use zerocopy::{ByteOrder, LE};

use super::{ByteSize, Offset, Result};

/// A Parser is something parsing data from a [u8]
pub trait Parser {
    fn read_slice(&mut self, size: usize) -> Result<Cow<[u8]>>;
    fn read_data(&mut self, buf: &mut [u8]) -> Result<()>;
    #[cfg(test)]
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

    fn read_isized(&mut self, size: ByteSize) -> Result<i64> {
        let size = size as usize;
        let slice = self.read_slice(size)?;
        Ok(LE::read_int(&slice, size))
    }
}

/// A RandomParser is something parsing data from a [u8] at random position
pub trait RandomParser {
    type Parser<'a>: Parser
    where
        Self: 'a;

    fn create_parser(&self, offset: Offset) -> Result<Self::Parser<'_>>;
    fn read_slice(&self, offset: Offset, size: usize) -> Result<Cow<[u8]>>;
    fn read_data(&self, offset: Offset, buf: &mut [u8]) -> std::io::Result<()>;

    fn global_offset(&self) -> Offset;
    fn read_u8(&self, offset: Offset) -> Result<u8> {
        let slice = self.read_slice(offset, 1)?;
        Ok(slice[0])
    }

    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let slice = self.read_slice(offset, 2)?;
        Ok(LE::read_u16(&slice))
    }

    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let slice = self.read_slice(offset, 4)?;
        Ok(LE::read_u32(&slice))
    }

    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let slice = self.read_slice(offset, 8)?;
        Ok(LE::read_u64(&slice))
    }

    fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        let size = size as usize;
        let slice = self.read_slice(offset, size)?;
        Ok(LE::read_uint(&slice, size))
    }

    fn read_i8(&self, offset: Offset) -> Result<i8> {
        let slice = self.read_slice(offset, 1)?;
        Ok(slice[0] as i8)
    }

    fn read_i16(&self, offset: Offset) -> Result<i16> {
        let slice = self.read_slice(offset, 2)?;
        Ok(LE::read_i16(&slice))
    }

    fn read_i32(&self, offset: Offset) -> Result<i32> {
        let slice = self.read_slice(offset, 4)?;
        Ok(LE::read_i32(&slice))
    }

    fn read_i64(&self, offset: Offset) -> Result<i64> {
        let slice = self.read_slice(offset, 8)?;
        Ok(LE::read_i64(&slice))
    }

    fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        let size = size as usize;
        let slice = self.read_slice(offset, size)?;
        Ok(LE::read_int(&slice, size))
    }
}

pub struct SliceParser<'a> {
    slice: Cow<'a, [u8]>,
    global_offset: Offset,
    offset: usize,
}

impl<'a> SliceParser<'a> {
    pub(crate) fn new(slice: Cow<'a, [u8]>, global_offset: Offset) -> Self {
        Self {
            slice,
            global_offset,
            offset: 0,
        }
    }
}

impl Parser for SliceParser<'_> {
    fn read_slice(&mut self, size: usize) -> Result<Cow<[u8]>> {
        if self.slice.len() < size + self.offset {
            return Err(format_error!(format!(
                "Out of slice. {size}({}) > {}",
                self.offset,
                self.slice.len()
            )));
        }
        let slice = &self.slice[self.offset..self.offset + size];
        self.offset += size;
        Ok(Cow::Borrowed(slice))
    }

    fn read_data(&mut self, buf: &mut [u8]) -> Result<()> {
        if self.slice.len() < buf.len() + self.offset {
            return Err(format_error!(format!(
                "Out of slice. {}({}) > {}",
                buf.len(),
                self.offset,
                self.slice.len()
            )));
        }
        buf.copy_from_slice(&self.slice[self.offset..self.offset + buf.len()]);
        self.offset += buf.len();
        Ok(())
    }

    fn skip(&mut self, size: usize) -> Result<()> {
        if self.slice.len() < size + self.offset {
            return Err(format_error!(format!(
                "Out of slice. {size}({}) > {}",
                self.offset,
                self.slice.len()
            )));
        }
        self.offset += size;
        Ok(())
    }

    #[cfg(test)]
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

pub(crate) trait SizedParsable: Parsable {
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

pub(crate) trait RandomParsable {
    type Output;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self::Output>
    where
        Self::Output: Sized;
}

impl RandomParsable for u8 {
    type Output = Self;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        parser.read_u8(offset)
    }
}
impl RandomParsable for u16 {
    type Output = Self;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        parser.read_u16(offset)
    }
}
impl RandomParsable for u32 {
    type Output = Self;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        parser.read_u32(offset)
    }
}
impl RandomParsable for u64 {
    type Output = Self;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        parser.read_u64(offset)
    }
}
