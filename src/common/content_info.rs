use crate::bases::*;

#[derive(Debug)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize))]
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

impl Parsable for ContentInfo {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let v = parser.read_u32()?;
        let blob_index = (v & 0xFFF) as u16;
        let cluster_index = v >> 12;
        Ok(Self {
            cluster_index: cluster_index.into(),
            blob_index: blob_index.into(),
        })
    }
}

impl SizedParsable for ContentInfo {
    const SIZE: usize = 4;
}

impl Serializable for ContentInfo {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let data = (self.cluster_index.into_u32() << 12) + (self.blob_index.into_u32() & 0xFFF_u32);
        ser.write_u32(data)
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for ContentInfo {
    fn header_footer(&self) -> Option<(String, String)> {
        Some(("ContentInfo(".to_string(), ")".to_string()))
    }

    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        out.field("Cluster index", &self.cluster_index.into_u64())?;
        out.field("Blob index", &self.blob_index.into_u64())
    }
}

#[cfg(feature = "explorable")]
impl graphex::Node for ContentInfo {
    fn display(&self) -> &dyn graphex::Display {
        self
    }

    fn serde(&self) -> Option<&dyn erased_serde::Serialize> {
        Some(self)
    }
}
