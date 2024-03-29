use crate::bases::*;

#[derive(Debug)]
pub struct ContentInfo {
    pub cluster_index: ClusterIdx,
    pub blob_index: BlobIdx,
}

impl ContentInfo {
    pub fn new(cluster_index: ClusterIdx, blob_index: BlobIdx) -> Self {
        Self {
            cluster_index,
            blob_index,
        }
    }
}

impl Producable for ContentInfo {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let v = flux.read_u32()?;
        let blob_index = (v & 0xFFF) as u16;
        let cluster_index = v >> 12;
        Ok(Self {
            cluster_index: cluster_index.into(),
            blob_index: blob_index.into(),
        })
    }
}

impl SizedProducable for ContentInfo {
    const SIZE: usize = 4;
}

impl Writable for ContentInfo {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let data = (self.cluster_index.into_u32() << 12) + (self.blob_index.into_u32() & 0xFFF_u32);
        stream.write_u32(data)
    }
}
