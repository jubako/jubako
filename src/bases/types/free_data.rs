use crate::bases::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct FreeData<const N: usize>([u8; N]);

impl<const N: usize> Producable for FreeData<N> {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let mut s = [0; N];
        flux.read_exact(s.as_mut_slice())?;
        Ok(Self(s))
    }
}
impl<const N: usize> SizedProducable for FreeData<N> {
    const SIZE: usize = N;
}

impl<const N: usize> Writable for FreeData<N> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_data(&self.0)
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

pub type DirectoryPackFreeData = FreeData<31>;
pub type ContentPackFreeData = FreeData<40>;
pub type ManifestPackFreeData = FreeData<55>;
pub type PackInfoFreeData = FreeData<103>;
