use std::ops::Deref;

use crate::bases::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
#[repr(transparent)]
pub struct VendorId([u8; 4]);

impl VendorId {
    pub const fn new(v: [u8; 4]) -> Self {
        Self(v)
    }
}

impl Deref for VendorId {
    type Target = [u8; 4];
    fn deref(&self) -> &[u8; 4] {
        &self.0
    }
}

#[cfg(feature = "explorable_serde")]
impl serde::Serialize for VendorId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct("VendorId", &String::from_utf8_lossy(&self.0))
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for VendorId {
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        out.write_str(&format!("{}", String::from_utf8_lossy(&self.0)))
    }
}

impl Parsable for VendorId {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let mut s = [0; 4];
        parser.read_data(s.as_mut_slice())?;
        Ok(Self(s))
    }
}
impl SizedParsable for VendorId {
    const SIZE: usize = 4;
}

impl Serializable for VendorId {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_data(&self.0)
    }
}

impl From<[u8; 4]> for VendorId {
    fn from(input: [u8; 4]) -> Self {
        Self(input)
    }
}
