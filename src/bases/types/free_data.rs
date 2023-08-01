use crate::bases::*;

pub type FreeData<const N: usize> = [u8; N];

impl<const N: usize> Producable for FreeData<N> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let mut s = [0; N];
        flux.read_exact(s.as_mut_slice())?;
        Ok(s)
    }
}
impl<const N: usize> SizedProducable for FreeData<N> {
    const SIZE: usize = N;
}

impl<const N: usize> Writable for FreeData<N> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_data(self.as_slice())
    }
}

pub type DirectoryPackFreeData = FreeData<31>;
pub type ContentPackFreeData = FreeData<40>;
pub type ManifestPackFreeData = FreeData<55>;
pub type PackInfoFreeData = FreeData<103>;
