use crate::bases::*;
use std::fmt;
use std::ops::{Add, AddAssign};

/// A count of object.
/// All count object can be stored in a u32.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize), serde(transparent))]
#[repr(transparent)]
pub struct Count<T>(pub T);

impl<T> From<T> for Count<T> {
    fn from(v: T) -> Count<T> {
        Count(v)
    }
}

impl<T> Add<T> for Count<T>
where
    T: Add<Output = T>,
{
    type Output = Self;
    fn add(self, other: T) -> Self {
        Count(self.0 + other)
    }
}

impl<T> AddAssign<T> for Count<T>
where
    T: AddAssign,
{
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs;
    }
}

impl<T> fmt::Display for Count<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Count : {}", self.0)
    }
}

impl<T> SizedParsable for Count<T>
where
    Count<T>: Parsable,
{
    const SIZE: usize = std::mem::size_of::<T>();
}

impl Parsable for Count<u8> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u8()?.into())
    }
}

impl Parsable for Count<u16> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u16()?.into())
    }
}

impl Parsable for Count<u32> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u32()?.into())
    }
}

impl Parsable for Count<u64> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u64()?.into())
    }
}

impl Serializable for Count<u8> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u8(self.0)
    }
}
impl Serializable for Count<u16> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u16(self.0)
    }
}
impl Serializable for Count<u32> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u32(self.0)
    }
}
impl Serializable for Count<u64> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u64(self.0)
    }
}
