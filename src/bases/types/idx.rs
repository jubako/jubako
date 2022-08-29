use crate::bases::*;
use std::fmt;
use std::ops::Add;

/// A index of a object.
/// All count object can be stored in a u32.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct Idx<T>(pub T);

impl Producable for Idx<u32> {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u32()?.into())
    }
}
impl SizedProducable for Idx<u32> {
    type Size = typenum::U4;
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
        write!(f, "Idx({})", self.0)
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

impl<T> From<T> for Idx<T> {
    fn from(v: T) -> Idx<T> {
        Idx(v)
    }
}

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
