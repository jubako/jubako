use crate::bases::*;
use std::fmt;
use std::ops::{Add, AddAssign, Sub};

/// A size used in Jubako.
/// We handling content in 64 bits space.
/// We cannot use a usize as it is arch dependent.
/// Let's define our own type.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize), serde(transparent))]
#[repr(transparent)]
pub struct Size(u64);

impl Size {
    pub const fn new(s: u64) -> Self {
        Self(s)
    }
    pub const fn zero() -> Self {
        Self(0)
    }
    #[inline]
    pub const fn into_u64(self) -> u64 {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Parsable for Size {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok(parser.read_u64()?.into())
    }
}
impl SizedParsable for Size {
    const SIZE: usize = 8;
}
impl Serializable for Size {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u64(self.0)
    }
}

impl From<ASize> for Size {
    fn from(v: ASize) -> Size {
        v.into_u64().into()
    }
}

impl From<Offset> for Size {
    fn from(v: Offset) -> Size {
        v.into_u64().into()
    }
}

impl From<u64> for Size {
    fn from(v: u64) -> Size {
        Size(v)
    }
}

impl From<usize> for Size {
    fn from(v: usize) -> Size {
        // We are compiling on 32 or 64 bits.
        Size(v as u64)
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Size : {}", self.0)
    }
}

impl Add<Size> for Size {
    type Output = Self;
    fn add(self, other: Size) -> Size {
        Size(self.0 + other.0)
    }
}

impl Add<u64> for Size {
    type Output = Self;
    fn add(self, other: u64) -> Size {
        Size(self.0 + other)
    }
}

impl AddAssign<u64> for Size {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}

impl AddAssign<usize> for Size {
    fn add_assign(&mut self, other: usize) {
        self.0 += other as u64;
    }
}

impl Sub<Size> for Size {
    type Output = Self;
    fn sub(self, other: Size) -> Size {
        Size(self.0 - other.0)
    }
}

impl std::ops::Mul<u64> for Size {
    type Output = Self;
    fn mul(self, other: u64) -> Size {
        Size(self.0 * other)
    }
}
