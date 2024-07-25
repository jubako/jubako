use crate::bases::*;
use crate::common::CompressionType;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ClusterHeader {
    pub compression: CompressionType,
    pub offset_size: ByteSize,
    pub blob_count: BlobCount,
}

impl ClusterHeader {
    pub fn new(compression: CompressionType, offset_size: ByteSize, blob_count: BlobCount) -> Self {
        Self {
            compression,
            offset_size,
            blob_count,
        }
    }
}

impl Parsable for ClusterHeader {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let compression = CompressionType::parse(parser)?;
        let offset_size = ByteSize::parse(parser)?;
        let blob_count = Count::<u16>::parse(parser)?.into();
        Ok(ClusterHeader {
            compression,
            offset_size,
            blob_count,
        })
    }
}

impl Serializable for ClusterHeader {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += self.compression.serialize(ser)?;
        written += self.offset_size.serialize(ser)?;
        written += self.blob_count.serialize(ser)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clusterheader() {
        let reader = CheckReader::from(vec![
            0x00, // compression
            0x01, // offset_size
            0x02, 0x00, // blob_count
        ]);
        let cluster_header = reader
            .parse_in::<ClusterHeader>(Offset::zero(), Size::new(4))
            .unwrap();
        assert_eq!(
            cluster_header,
            ClusterHeader {
                compression: CompressionType::None,
                offset_size: ByteSize::U1,
                blob_count: BlobCount::from(2),
            }
        );
    }
}
