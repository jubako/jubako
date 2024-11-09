use crate::bases::*;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Sub};

/// A addressable or arch dependent size used in Jubako.
/// We handling content in 64 bits space.
/// We cannot use a usize as it is arch dependent.
/// Let's define our own type.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default)]
#[cfg_attr(
    feature = "explorable_serde",
    derive(serde::Serialize),
    serde(transparent)
)]
#[repr(transparent)]
pub(crate) struct ASize(usize);

impl ASize {
    #[inline]
    pub const fn new(s: usize) -> Self {
        Self(s)
    }

    #[inline]
    pub const fn into_u64(self) -> u64 {
        self.0 as u64
    }

    #[inline]
    pub const fn into_usize(self) -> usize {
        self.0
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl From<usize> for ASize {
    fn from(v: usize) -> ASize {
        // We are compiling on 32 or 64 bits.
        ASize(v)
    }
}

impl TryFrom<Size> for ASize {
    type Error = std::num::TryFromIntError;
    fn try_from(v: Size) -> std::result::Result<Self, Self::Error> {
        // We are compiling on 32 or 64 bits.
        Ok(ASize::new(v.into_u64().try_into()?))
    }
}

impl fmt::Display for ASize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Adressable Size : {}", self.0)
    }
}

impl Mul<usize> for ASize {
    type Output = Self;
    fn mul(self, other: usize) -> ASize {
        ASize(self.0 * other)
    }
}

impl Add<ASize> for ASize {
    type Output = Self;
    fn add(self, other: ASize) -> ASize {
        ASize(self.0 + other.0)
    }
}

impl Add<usize> for ASize {
    type Output = Self;
    fn add(self, other: usize) -> ASize {
        ASize(self.0 + other)
    }
}

impl AddAssign<usize> for ASize {
    fn add_assign(&mut self, other: usize) {
        self.0 += other;
    }
}

impl Sub<ASize> for ASize {
    type Output = Self;
    fn sub(self, other: ASize) -> ASize {
        ASize(self.0 - other.0)
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for ASize {
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        writeln!(out, "{}", self.into_u64())
    }
}
