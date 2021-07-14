use crate::bases::*;
use lzma::LzmaError;
use std::fmt;
use std::ops::{Add, AddAssign, Sub};

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    FormatError,
    ArgError,
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::IOError(e)
    }
}

impl From<lzma::LzmaError> for Error {
    fn from(e: LzmaError) -> Error {
        match e {
            LzmaError::Io(e) => Error::IOError(e),
            _ => Error::FormatError,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IOError(e) => write!(f, "IO Error {}", e),
            Error::FormatError => write!(f, "Arx format error"),
            Error::ArgError => write!(f, "Invalid argument"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// A offset used in xar.
/// We handling content in 64 bits space.
/// We cannot use a usize as it is arch dependent.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct Offset(pub u64);

impl Offset {
    pub fn is_valid(self, s: Size) -> bool {
        self.0 <= s.0
    }
}

impl Producable for Offset {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
    }
}

impl From<Size> for Offset {
    fn from(v: Size) -> Offset {
        v.0.into()
    }
}

impl From<u64> for Offset {
    fn from(v: u64) -> Offset {
        Offset(v)
    }
}

impl Add<usize> for Offset {
    type Output = Offset;
    fn add(self, other: usize) -> Offset {
        Offset(self.0.checked_add(other as u64).unwrap())
    }
}

impl Add<Size> for Offset {
    type Output = Offset;
    fn add(self, other: Size) -> Offset {
        Offset(self.0.checked_add(other.0).unwrap())
    }
}

impl Add for Offset {
    type Output = Offset;
    fn add(self, other: Offset) -> Offset {
        Offset(self.0.checked_add(other.0).unwrap())
    }
}

impl AddAssign<usize> for Offset {
    fn add_assign(&mut self, other: usize) {
        self.0 = self.0.checked_add(other as u64).unwrap();
    }
}

impl AddAssign for Offset {
    fn add_assign(&mut self, other: Offset) {
        self.0 = self.0.checked_add(other.0).unwrap();
    }
}

impl Sub for Offset {
    type Output = Size;
    fn sub(self, other: Offset) -> Size {
        Size(self.0.checked_sub(other.0).unwrap())
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Offset({})", self.0)
    }
}

/// A size used in xar.
/// We handling content in 64 bits space.
/// We cannot use a usize as it is arch dependent.
/// Let's define our own type.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct Size(pub u64);

impl Producable for Size {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
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

/// The end of a buffer.
pub enum End {
    Offset(Offset),
    Size(Size),
    None,
}

impl fmt::Display for End {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            End::None => write!(f, "End::None"),
            End::Offset(o) => write!(f, "End::{}", o),
            End::Size(s) => write!(f, "End::{}", s),
        }
    }
}

/// A count of object.
/// All count object can be stored in a u32.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct Count<T>(pub T);

impl<T> From<T> for Count<T> {
    fn from(v: T) -> Count<T> {
        Count(v)
    }
}

impl Producable for Count<u8> {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u8()?.into())
    }
}

impl Producable for Count<u16> {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u16()?.into())
    }
}

impl Producable for Count<u32> {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u32()?.into())
    }
}

impl Producable for Count<u64> {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
    }
}

impl<T:fmt::Display> fmt::Display for Count<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Count({})", self.0)
    }
}

/// A index of a object.
/// All count object can be stored in a u32.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct Idx<T>(pub T);

impl<T> Idx<T>
where
    T: std::cmp::PartialOrd,
{
    pub fn is_valid(&self, s: Count<T>) -> bool {
        self.0 < s.0
    }
}

impl<T> From<T> for Idx<T> {
    fn from(v: T) -> Idx<T> {
        Idx(v)
    }
}

impl<T:fmt::Display> fmt::Display for Idx<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Idx({})", self.0)
    }
}

/// This is somehow the same as std::ops::Index
/// but with a output by value and not by ref.
pub trait Index<Idx> {
    type OutputType;
    fn index(&self, idx: Idx) -> Self::OutputType;
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
