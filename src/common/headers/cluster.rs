use crate::bases::*;
use crate::common::CompressionType;

#[derive(Debug, PartialEq, Eq)]
pub struct ClusterHeader {
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

impl Producable for ClusterHeader {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let compression = CompressionType::produce(flux)?;
        let offset_size = ByteSize::produce(flux)?;
        let blob_count = Count::<u16>::produce(flux)?.into();
        Ok(ClusterHeader {
            compression,
            offset_size,
            blob_count,
        })
    }
}

impl Writable for ClusterHeader {
    fn write(&self, out_stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += self.compression.write(out_stream)?;
        written += self.offset_size.write(out_stream)?;
        written += self.blob_count.write(out_stream)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clusterheader() {
        let reader = Reader::from(vec![
            0x00, // compression
            0x01, // offset_size
            0x00, 0x02, // blob_count
        ]);
        let mut flux = reader.create_flux_all();
        assert_eq!(
            ClusterHeader::produce(&mut flux).unwrap(),
            ClusterHeader {
                compression: CompressionType::None,
                offset_size: ByteSize::U1,
                blob_count: BlobCount::from(2),
            }
        );
    }
}
