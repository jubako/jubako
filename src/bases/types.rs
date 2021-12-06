use crate::bases::*;
use generic_array::{ArrayLength, GenericArray};
use lzma::LzmaError;
use std::fmt;
use std::ops::{Add, AddAssign, Sub};
use std::string::FromUtf8Error;

#[derive(Debug)]
pub struct FormatError {
    what: String,
    where_: Option<Offset>,
}

impl FormatError {
    pub fn new(what: &str, where_: Option<Offset>) -> Error {
        FormatError {
            what: what.into(),
            where_,
        }
        .into()
    }
}

//#[macro_export]
macro_rules! format_error {
    ($what:expr, $stream:ident) => {
        FormatError::new($what, Some($stream.global_offset()))
    };
    ($what:expr) => {
        FormatError::new($what, None)
    };
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.where_ {
            None => write!(f, "{}", self.what),
            Some(w) => write!(f, "{} at offset {}", self.what, w),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Format(FormatError),
    Arg,
    Other(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<FormatError> for Error {
    fn from(e: FormatError) -> Error {
        Error::Format(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_e: FromUtf8Error) -> Error {
        FormatError::new("Utf8DecodingError", None)
    }
}

impl From<lzma::LzmaError> for Error {
    fn from(e: LzmaError) -> Error {
        match e {
            LzmaError::Io(e) => Error::Io(e),
            _ => FormatError::new("Lzma compression error", None),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO Error {}", e),
            Error::Format(e) => write!(f, "Jubako format error {}", e),
            Error::Arg => write!(f, "Invalid argument"),
            Error::Other(e) => write!(f, "Unknown error : {}", e),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// A offset used Jubako.
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
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
    }
}
impl SizedProducable for Offset {
    type Size = typenum::U8;
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
    type Output = Self;
    fn add(self, other: usize) -> Offset {
        Offset(self.0.checked_add(other as u64).unwrap())
    }
}

impl Add<Size> for Offset {
    type Output = Self;
    fn add(self, other: Size) -> Offset {
        Offset(self.0.checked_add(other.0).unwrap())
    }
}

impl Add for Offset {
    type Output = Self;
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

#[derive(Debug, Clone, Copy)]
pub struct SizedOffset {
    pub size: Size,
    pub offset: Offset,
}

impl SizedOffset {
    pub fn new(size: Size, offset: Offset) -> Self {
        Self { size, offset }
    }
}

impl SizedProducable for SizedOffset {
    type Size = typenum::U8;
}

impl Producable for SizedOffset {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let data = stream.read_u64()?;
        let offset = Offset(data & 0xFF_FF_FF_FF_FF_FF_u64);
        let size = Size(data >> 48);
        Ok(Self::new(size, offset))
    }
}

/// The end of a buffer.
pub enum End {
    Offset(Offset),
    Size(Size),
    None,
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

impl<T> Add<T> for Count<T>
where
    T: std::ops::Add<Output = T>,
{
    type Output = Self;
    fn add(self, other: T) -> Self {
        Count(self.0 + other)
    }
}

impl<T> fmt::Display for Count<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Count({})", self.0)
    }
}

impl Producable for Count<u8> {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u8()?.into())
    }
}
impl SizedProducable for Count<u8> {
    type Size = typenum::U1;
}

impl Producable for Count<u16> {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u16()?.into())
    }
}
impl SizedProducable for Count<u16> {
    type Size = typenum::U2;
}

impl Producable for Count<u32> {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u32()?.into())
    }
}
impl SizedProducable for Count<u32> {
    type Size = typenum::U4;
}

impl Producable for Count<u64> {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(stream.read_u64()?.into())
    }
}
impl SizedProducable for Count<u64> {
    type Size = typenum::U8;
}

/// A index of a object.
/// All count object can be stored in a u32.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
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

pub type FreeData<N> = GenericArray<u8, N>;

impl<N: ArrayLength<u8>> Producable for FreeData<N> {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let mut s = GenericArray::default();
        stream.read_exact(s.as_mut_slice())?;
        Ok(s)
    }
}
impl<N: ArrayLength<u8>> SizedProducable for FreeData<N> {
    type Size = N;
}

pub struct PString {}

impl Producable for PString {
    type Output = Vec<u8>;
    fn produce(stream: &mut dyn Stream) -> Result<Vec<u8>> {
        let size = stream.read_u8()?;
        stream.read_vec(size as usize)
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

    #[test_case(&[0x00] => "")]
    #[test_case(&[0x01, 72] => "H")]
    #[test_case(&[0x02, 72, 101] => "He")]
    #[test_case(&[0x03, 72, 0xC3, 0xA9] => "Hé")]
    fn test_pstring(source: &[u8]) -> String {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        String::from_utf8(PString::produce(stream.as_mut()).unwrap()).unwrap()
    }
}
