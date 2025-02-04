use crate::bases::*;
use std::fmt;
use std::ops::{Add, AddAssign, Sub};

/// A offset used Jubako.
/// We handling content in 64 bits space.
/// We cannot use a usize as it is arch dependent.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default)]
#[cfg_attr(
    feature = "explorable_serde",
    derive(serde::Serialize),
    serde(transparent)
)]
#[repr(transparent)]
pub struct Offset(u64);

impl Offset {
    #[inline]
    pub fn is_valid(self, s: Size) -> bool {
        self.0 <= s.into_u64()
    }

    #[inline]
    pub fn into_u64(self) -> u64 {
        self.0
    }

    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    #[inline]
    pub fn force_into_usize(self) -> usize {
        assert!(self.0 <= usize::MAX as u64);
        self.0 as usize
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn new(s: u64) -> Self {
        Self(s)
    }
}

impl Parsable for Offset {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u64()?.into())
    }
}
impl SizedParsable for Offset {
    const SIZE: usize = 8;
}

impl Serializable for Offset {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u64(self.0)
    }
}

impl From<Size> for Offset {
    fn from(v: Size) -> Offset {
        v.into_u64().into()
    }
}

impl From<ASize> for Offset {
    fn from(v: ASize) -> Offset {
        v.into_u64().into()
    }
}

impl From<u64> for Offset {
    fn from(v: u64) -> Offset {
        Offset(v)
    }
}

impl From<usize> for Offset {
    fn from(v: usize) -> Offset {
        Offset(v as u64)
    }
}

impl Add<usize> for Offset {
    type Output = Self;
    fn add(self, other: usize) -> Offset {
        Offset(self.0 + other as u64)
    }
}

impl Add<Size> for Offset {
    type Output = Self;
    fn add(self, other: Size) -> Offset {
        Offset(self.0 + other.into_u64())
    }
}

impl Add<ASize> for Offset {
    type Output = Self;
    fn add(self, other: ASize) -> Offset {
        Offset(self.0 + other.into_u64())
    }
}

impl Add for Offset {
    type Output = Self;
    fn add(self, other: Offset) -> Offset {
        Offset(self.0 + other.0)
    }
}

impl AddAssign<usize> for Offset {
    fn add_assign(&mut self, other: usize) {
        self.0 += other as u64;
    }
}

impl AddAssign<Size> for Offset {
    fn add_assign(&mut self, other: Size) {
        self.0 += other.into_u64();
    }
}

impl AddAssign<ASize> for Offset {
    fn add_assign(&mut self, other: ASize) {
        self.0 += other.into_u64();
    }
}

impl AddAssign for Offset {
    fn add_assign(&mut self, other: Offset) {
        self.0 += other.0;
    }
}

impl Sub for Offset {
    type Output = Size;
    fn sub(self, other: Offset) -> Size {
        Size::from(self.0 - other.0)
    }
}

impl Sub<Size> for Offset {
    type Output = Offset;
    fn sub(self, other: Size) -> Offset {
        Offset::from(self.0 - other.into_u64())
    }
}

impl Sub<ASize> for Offset {
    type Output = Offset;
    fn sub(self, other: ASize) -> Offset {
        Offset::from(self.0 - other.into_u64())
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Offset : {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(0, 0 => true)]
    #[test_case(0, 1 => true)]
    #[test_case(1, 1 => true)]
    #[test_case(1, 0 => false)]
    #[test_case(254, 255 => true)]
    #[test_case(255, 255 => true)]
    #[test_case(256, 255 => false)]
    fn test_offset_is_valid(o: u64, s: u64) -> bool {
        Offset(o).is_valid(s.into())
    }
}
