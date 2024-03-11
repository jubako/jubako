use crate::bases::*;
use std::fmt;
use std::ops::{Add, AddAssign};

/// A count of object.
/// All count object can be stored in a u32.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, serde::Serialize)]
#[serde(transparent)]
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

impl<T> SizedProducable for Count<T>
where
    Count<T>: Producable,
{
    const SIZE: usize = std::mem::size_of::<T>();
}

impl Producable for Count<u8> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        Ok(flux.read_u8()?.into())
    }
}

impl Producable for Count<u16> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        Ok(flux.read_u16()?.into())
    }
}

impl Producable for Count<u32> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        Ok(flux.read_u32()?.into())
    }
}

impl Producable for Count<u64> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        Ok(flux.read_u64()?.into())
    }
}

impl Writable for Count<u8> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u8(self.0)
    }
}
impl Writable for Count<u16> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u16(self.0)
    }
}
impl Writable for Count<u32> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u32(self.0)
    }
}
impl Writable for Count<u64> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u64(self.0)
    }
}
