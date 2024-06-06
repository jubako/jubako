use crate::bases::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
#[repr(transparent)]
pub struct VendorId([u8; 4]);

impl VendorId {
    pub const fn new(v: [u8; 4]) -> Self {
        Self(v)
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for VendorId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct("VendorId", &String::from_utf8_lossy(&self.0))
    }
}

impl Producable for VendorId {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let mut s = [0; 4];
        flux.read_exact(s.as_mut_slice())?;
        Ok(Self(s))
    }
}
impl SizedProducable for VendorId {
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
