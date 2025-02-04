use crate::bases::*;
use std::fmt;
use std::ops::{Add, AddAssign};

/// A index of a object.
/// All count object can be stored in a u32.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default, Hash)]
#[cfg_attr(
    feature = "explorable_serde",
    derive(serde::Serialize),
    serde(transparent)
)]
#[repr(transparent)]
pub struct Idx<T>(T);

impl<T> Idx<T> {
    #[inline]
    pub(crate) fn new(v: T) -> Self {
        Self(v)
    }
    #[inline]
    pub(crate) fn into_base(self) -> T {
        self.0
    }
}

impl Parsable for Idx<u8> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u8()?.into())
    }
}

impl Parsable for Idx<u16> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u16()?.into())
    }
}

impl Parsable for Idx<u32> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u32()?.into())
    }
}

impl Parsable for Idx<u64> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u64()?.into())
    }
}

impl RandomParsable for Idx<u8> {
    type Output = Self;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self> {
        Ok(parser.read_u8(offset)?.into())
    }
}

impl RandomParsable for Idx<u16> {
    type Output = Self;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self> {
        Ok(parser.read_u16(offset)?.into())
    }
}

impl RandomParsable for Idx<u32> {
    type Output = Self;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self> {
        Ok(parser.read_u32(offset)?.into())
    }
}

impl RandomParsable for Idx<u64> {
    type Output = Self;
    fn rparse(parser: &impl RandomParser, offset: Offset) -> Result<Self> {
        Ok(parser.read_u64(offset)?.into())
    }
}

impl<T> SizedParsable for Idx<T>
where
    Idx<T>: Parsable,
{
    const SIZE: usize = std::mem::size_of::<T>();
}

impl Serializable for Idx<u8> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u8(self.0)
    }
}
impl Serializable for Idx<u32> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u32(self.0)
    }
}

impl<T> fmt::Display for Idx<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Idx : {}", self.0)
    }
}

impl<T> Idx<T>
where
    T: std::cmp::PartialOrd,
{
    pub fn is_valid(&self, s: Count<T>) -> bool {
        self.0 < s.into_base()
    }
}

impl<T> Add for Idx<T>
where
    T: std::ops::Add<Output = T>,
{
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Idx(self.0 + other.0)
    }
}

impl<T> Add<Count<T>> for Idx<T>
where
    T: std::ops::Add<Output = T>,
{
    type Output = Self;
    fn add(self, other: Count<T>) -> Self {
        Idx(self.0 + other.into_base())
    }
}

impl<T> AddAssign<T> for Idx<T>
where
    T: std::ops::AddAssign,
{
    fn add_assign(&mut self, other: T) {
        self.0 += other;
    }
}

impl<T> std::ops::Deref for Idx<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for Idx<T> {
    fn from(v: T) -> Idx<T> {
        Idx(v)
    }
}
/*
impl Into<usize> for Idx<u32> {
    fn into(self) -> usize {
        self.0 as usize
    }
}
*/
/// This is somehow the same as std::ops::Index
/// but with a output by value and not by ref.
pub trait IndexTrait<Idx> {
    type OutputType;
    fn index(&self, idx: Idx) -> Self::OutputType;
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(0, 0 => false)]
    #[test_case(0, 1 => true)]
    #[test_case(1, 1 => false)]
    #[test_case(1, 0 => false)]
    #[test_case(254, 255 => true)]
    #[test_case(255, 255 => false)]
    #[test_case(256, 255 => false)]
    fn test_index_is_valid(o: u64, s: u64) -> bool {
        Idx(o).is_valid(s.into())
    }
}
