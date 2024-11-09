use crate::bases::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(transparent)]
pub struct FreeData<const N: usize>([u8; N]);

impl<const N: usize> Parsable for FreeData<N> {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let mut s = [0; N];
        parser.read_data(s.as_mut_slice())?;
        Ok(Self(s))
    }
}
impl<const N: usize> SizedParsable for FreeData<N> {
    const SIZE: usize = N;
}

#[cfg(feature = "explorable_serde")]
impl<const N: usize> serde::Serialize for FreeData<N> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&String::from_utf8_lossy(&self.0))
    }
}

impl<const N: usize> Serializable for FreeData<N> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_data(&self.0)
    }
}

impl<const N: usize> Default for FreeData<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> std::ops::Deref for FreeData<N> {
    type Target = [u8; N];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> From<[u8; N]> for FreeData<N> {
    fn from(input: [u8; N]) -> Self {
        Self(input)
    }
}

#[cfg(feature = "explorable")]
impl<const N: usize> graphex::Display for FreeData<N> {
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        graphex::AsBytes(&self.0).print(out)
    }
}

pub type PackFreeData = FreeData<24>;
pub type IndexFreeData = FreeData<4>;
