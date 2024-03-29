use crate::bases::*;
use std::fmt;
use std::ops::{Add, AddAssign, Sub};

/// A offset used Jubako.
/// We handling content in 64 bits space.
/// We cannot use a usize as it is arch dependent.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default)]
#[repr(transparent)]
pub struct Offset(u64);

impl Offset {
    pub fn is_valid(self, s: Size) -> bool {
        self.0 <= s.into_u64()
    }

    pub fn into_u64(self) -> u64 {
        self.0
    }

    #[cfg(target_pointer_width = "64")]
    pub fn into_usize(self) -> usize {
        self.0 as usize
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn new(s: u64) -> Self {
        Self(s)
    }
}

impl Producable for Offset {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        Ok(flux.read_u64()?.into())
    }
}
impl SizedProducable for Offset {
    const SIZE: usize = 8;
}

impl Writable for Offset {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u64(self.0)
    }
}

impl From<Size> for Offset {
    fn from(v: Size) -> Offset {
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
