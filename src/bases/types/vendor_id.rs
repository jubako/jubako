use crate::bases::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(transparent)]
pub struct VendorId([u8; 4]);

impl VendorId {
    pub const fn new(v: [u8; 4]) -> Self {
        Self(v)
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

impl Writable for VendorId {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_data(&self.0)
    }
}

impl Default for VendorId {
    fn default() -> Self {
        Self([0; 4])
    }
}

impl From<[u8; 4]> for VendorId {
    fn from(input: [u8; 4]) -> Self {
        Self(input)
    }
}
