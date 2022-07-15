use crate::bases::*;
use typenum::U4;

#[derive(Debug)]
pub struct EntryInfo {
    pub cluster_index: Idx<u32>,
    pub blob_index: Idx<u16>,
}

impl EntryInfo {
    pub fn new(cluster_index: Idx<u32>, blob_index: Idx<u16>) -> Self {
        Self {
            cluster_index,
            blob_index,
        }
    }
}

impl Producable for EntryInfo {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let v = stream.read_u32()?;
        let blob_index = (v & 0xFFF) as u16;
        let cluster_index = v >> 12;
        Ok(EntryInfo {
            cluster_index: cluster_index.into(),
            blob_index: blob_index.into(),
        })
    }
}

impl SizedProducable for EntryInfo {
    type Size = U4;
}

impl Writable for EntryInfo {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        let data: u32 = (self.cluster_index.0 << 12) + (self.blob_index.0 & 0xFFF_u16) as u32;
        stream.write_u32(data)
    }
}
