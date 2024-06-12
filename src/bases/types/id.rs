use crate::bases::*;
use std::fmt;

/// A identifier for a object.
/// Identifier is somehow a simple integer, but without computation.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Default, Hash)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize), serde(transparent))]
#[repr(transparent)]
pub struct Id<T>(pub T);

impl<T> Id<T> {
    pub(crate) fn into_base(self) -> T {
        self.0
    }
}

impl Parsable for Id<u8> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u8()?.into())
    }
}

impl Parsable for Id<u16> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u16()?.into())
    }
}

impl<T> SizedParsable for Id<T>
where
    Id<T>: Parsable,
{
    const SIZE: usize = std::mem::size_of::<T>();
}

impl Serializable for Id<u8> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u8(self.0)
    }
}
impl Serializable for Id<u16> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u16(self.0)
    }
}
impl Serializable for Id<u32> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u32(self.0)
    }
}

impl<T> fmt::Display for Id<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id : {}", self.0)
    }
}

impl<T> From<T> for Id<T> {
    fn from(v: T) -> Id<T> {
        Id(v)
    }
}
