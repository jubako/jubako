use crate::bases::*;
use generic_array::{ArrayLength, GenericArray};
use typenum::{U103, U31, U40, U55};

pub type FreeData<N> = GenericArray<u8, N>;

impl<N: ArrayLength<u8>> Producable for FreeData<N> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let mut s = GenericArray::default();
        flux.read_exact(s.as_mut_slice())?;
        Ok(s)
    }
}
impl<N: ArrayLength<u8>> SizedProducable for FreeData<N> {
    type Size = N;
}
impl<N: ArrayLength<u8>> Writable for FreeData<N> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_data(self.as_slice())
    }
}

pub type FreeData31 = FreeData<U31>;
pub type FreeData40 = FreeData<U40>;
pub type FreeData55 = FreeData<U55>;
pub type FreeData103 = FreeData<U103>;
