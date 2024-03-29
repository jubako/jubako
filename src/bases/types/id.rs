use crate::bases::*;
use std::fmt;

/// A identifier for a object.
/// Identifier is somehow a simple integer, but without computation.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Default, Hash)]
#[repr(transparent)]
pub struct Id<T>(pub T);

impl<T> Id<T> {
    pub(crate) fn into_base(self) -> T {
        self.0
    }
}

impl Producable for Id<u8> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        Ok(flux.read_u8()?.into())
    }
}
impl SizedProducable for Id<u8> {
    const SIZE: usize = 1;
}
impl Writable for Id<u8> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u8(self.0)
    }
}
impl Writable for Id<u16> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u16(self.0)
    }
}
impl Writable for Id<u32> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u32(self.0)
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
