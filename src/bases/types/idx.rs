use crate::bases::*;
use std::fmt;
use std::ops::{Add, AddAssign};

/// A index of a object.
/// All count object can be stored in a u32.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default, Hash)]
#[repr(transparent)]
pub struct Idx<T>(pub T);

impl<T> Idx<T> {
    pub(crate) fn into_base(self) -> T {
        self.0
    }
}

impl Producable for Idx<u32> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        Ok(flux.read_u32()?.into())
    }
}
impl<T> SizedProducable for Idx<T>
where
    Idx<T>: Producable,
{
    const SIZE: usize = std::mem::size_of::<T>();
}

impl Writable for Idx<u8> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u8(self.0)
    }
}
impl Writable for Idx<u32> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u32(self.0)
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
        self.0 < s.0
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
        Idx(self.0 + other.0)
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
