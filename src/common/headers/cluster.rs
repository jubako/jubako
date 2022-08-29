use crate::bases::*;
use crate::common::CompressionType;

#[derive(Debug, PartialEq, Eq)]
pub struct ClusterHeader {
    pub compression: CompressionType,
    pub offset_size: u8,
    pub blob_count: Count<u16>,
}

impl ClusterHeader {
    pub fn new(compression: CompressionType, offset_size: u8, blob_count: Count<u16>) -> Self {
        Self {
            compression,
            offset_size,
            blob_count,
        }
    }
}

impl Producable for ClusterHeader {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let compression = CompressionType::produce(stream)?;
        let offset_size = stream.read_u8()?;
        let blob_count = Count::<u16>::produce(stream)?;
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
        written += out_stream.write_u8(self.offset_size)?;
        written += self.blob_count.write(out_stream)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clusterheader() {
        let reader = BufReader::new(
            vec![
                0x00, // compression
                0x01, // offset_size
                0x00, 0x02, // blob_count
            ],
            End::None,
        );
        let mut stream = reader.create_stream_all();
        assert_eq!(
            ClusterHeader::produce(stream.as_mut()).unwrap(),
            ClusterHeader {
                compression: CompressionType::None,
                offset_size: 1,
                blob_count: Count(2),
            }
        );
    }
}
