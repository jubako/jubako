use crate::bases::*;
use generic_array::{ArrayLength, GenericArray};

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
impl<N: ArrayLength<u8>> Writable for FreeData<N> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_data(self.as_slice())
    }
}
