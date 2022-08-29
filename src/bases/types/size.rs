use crate::bases::*;
use std::fmt;
use std::ops::Add;

/// A size used in Jubako.
/// We handling content in 64 bits space.
/// We cannot use a usize as it is arch dependent.
/// Let's define our own type.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct Size(pub u64);

impl Producable for Size {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
    }
}
impl SizedProducable for Size {
    type Size = typenum::U8;
}
impl Writable for Size {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u64(self.0)
    }
}

impl From<Offset> for Size {
    fn from(v: Offset) -> Size {
        v.0.into()
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
        write!(f, "Size({})", self.0)
    }
}

impl Add<Size> for Size {
    type Output = Self;
    fn add(self, other: Size) -> Size {
        Size(self.0.checked_add(other.0).unwrap())
    }
}
