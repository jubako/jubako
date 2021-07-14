use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompressionType {
    NONE = 0,
    LZ4 = 1,
    LZMA = 2,
    ZSTD = 3,
}

impl Producable for CompressionType {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        match stream.read_u8()? {
            0 => Ok(CompressionType::NONE),
            1 => Ok(CompressionType::LZ4),
            2 => Ok(CompressionType::LZMA),
            3 => Ok(CompressionType::ZSTD),
            _ => Err(Error::FormatError),
        }
    }
}

#[derive(Debug, PartialEq)]
struct ClusterHeader {
    compression: CompressionType,
    offset_size: u8,
    blob_count: Count<u16>,
    cluster_size: Size,
}

impl Producable for ClusterHeader {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let compression = CompressionType::produce(stream)?;
        let offset_size = stream.read_u8()?;
        let blob_count = Count::produce(stream)?;
        let cluster_size = Size::produce(stream)?;
        Ok(ClusterHeader {
            compression,
            offset_size,
            blob_count,
            cluster_size,
        })
    }
}

pub struct Cluster {
    blob_offsets: Vec<Offset>,
    reader: Box<dyn Reader>,
}

impl Cluster {
    pub fn new(reader: &dyn Reader) -> Result<Self> {
        let mut stream = reader.create_stream(Offset::from(0), End::None);
        let header = ClusterHeader::produce(stream.as_mut())?;
        let data_size: Size = stream.read_sized(header.offset_size.into())?.into();
        let mut blob_offsets: Vec<Offset> = Vec::with_capacity((header.blob_count.0 + 1) as usize);
        unsafe { blob_offsets.set_len((header.blob_count.0).into()) }
        let mut first = true;
        for elem in blob_offsets.iter_mut() {
            if first {
                *elem = 0.into();
                first = false;
            } else {
                *elem = stream.read_sized(header.offset_size.into())?.into();
            }
            assert!(elem.is_valid(data_size));
        }
        blob_offsets.push(data_size.into());
        let raw_reader =
            reader.create_sub_reader(stream.tell(), End::Offset(header.cluster_size.into()));
        let reader = match header.compression {
            CompressionType::NONE => {
                assert_eq!((stream.tell() + data_size).0, header.cluster_size.0);
                raw_reader
            }
            CompressionType::LZ4 => {
                let stream = raw_reader.create_stream(Offset(0), End::None);
                Box::new(Lz4Reader::new(lz4::Decoder::new(stream)?, data_size))
            }
            CompressionType::LZMA => {
                let stream = raw_reader.create_stream(Offset(0), End::None);
                Box::new(LzmaReader::new(
                    lzma::LzmaReader::new_decompressor(stream)?,
                    data_size,
                ))
            }
            CompressionType::ZSTD => {
                let stream = raw_reader.create_stream(Offset(0), End::None);
                Box::new(ZstdReader::new(zstd::Decoder::new(stream)?, data_size))
            }
        };
        Ok(Cluster {
            blob_offsets,
            reader,
        })
    }

    pub fn blob_count(&self) -> Count<u16> {
        Count::from((self.blob_offsets.len() - 1) as u16)
    }

    pub fn get_reader(&self, index: Idx<u16>) -> Result<Box<dyn Reader>> {
        let offset = self.blob_offsets[index.0 as usize];
        let end_offset = self.blob_offsets[(index.0 + 1) as usize];
        Ok(self
            .reader
            .create_sub_reader(offset, End::Offset(end_offset)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use test_case::test_case;

    #[test]
    fn test_compressiontype() {
        let reader = BufReader::new(vec![0x00, 0x01, 0x02, 0x03, 0x4, 0xFF], End::None);
        let mut stream = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            CompressionType::produce(stream.as_mut()).unwrap(),
            CompressionType::NONE
        );
        assert_eq!(
            CompressionType::produce(stream.as_mut()).unwrap(),
            CompressionType::LZ4
        );
        assert_eq!(
            CompressionType::produce(stream.as_mut()).unwrap(),
            CompressionType::LZMA
        );
        assert_eq!(
            CompressionType::produce(stream.as_mut()).unwrap(),
            CompressionType::ZSTD
        );
        assert_eq!(stream.tell(), Offset::from(4));
        assert!(CompressionType::produce(stream.as_mut()).is_err());
    }

    #[test]
    fn test_clusterheader() {
        let reader = BufReader::new(
            vec![
                0x00, // compression
                0x01, // offset_size
                0x00, 0x02, // blob_count
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // cluster_size
            ],
            End::None,
        );
        let mut stream = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            ClusterHeader::produce(stream.as_mut()).unwrap(),
            ClusterHeader {
                compression: CompressionType::NONE,
                offset_size: 1,
                blob_count: Count(2),
                cluster_size: Size(3)
            }
        );
    }

    fn create_cluster(comp: CompressionType, data: &[u8]) -> Vec<u8> {
        let cluster_size = (15 + data.len()) as u8; // Assume cluster_size is less than 256.
        let mut cluster_data = vec![
            comp as u8, // compression
            0x01,       // offset_size
            0x00, 0x03, // blob_count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, cluster_size, // cluster_size
            0x0f,         // Data size
            0x05,         // Offset of blob 1
            0x08,         // Offset of blob 2
        ];
        cluster_data.extend_from_slice(&data);
        assert_eq!(cluster_size as usize, cluster_data.len());
        cluster_data
    }

    fn create_raw_cluster() -> Vec<u8> {
        let raw_data = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        create_cluster(CompressionType::NONE, &raw_data)
    }

    fn create_lz4_cluster() -> Vec<u8> {
        let indata = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        let data = {
            let compressed_content = Vec::new();
            let mut encoder = lz4::EncoderBuilder::new()
                .level(16)
                .build(Cursor::new(compressed_content))
                .unwrap();
            let mut incursor = Cursor::new(indata);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            let (compressed_content, err) = encoder.finish();
            err.unwrap();
            compressed_content.into_inner()
        };
        create_cluster(CompressionType::LZ4, &data)
    }

    fn create_lzma_cluster() -> Vec<u8> {
        let indata = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        let data = {
            let compressed_content = Vec::new();
            let mut encoder =
                lzma::LzmaWriter::new_compressor(Cursor::new(compressed_content), 9).unwrap();
            let mut incursor = Cursor::new(indata);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        create_cluster(CompressionType::LZMA, &data)
    }

    fn create_zstd_cluster() -> Vec<u8> {
        let indata = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        let data = {
            let compressed_content = Vec::new();
            let mut encoder = zstd::Encoder::new(Cursor::new(compressed_content), 0).unwrap();
            let mut incursor = Cursor::new(indata);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        create_cluster(CompressionType::ZSTD, &data)
    }

    type ClusterCreator = fn() -> Vec<u8>;

    #[test_case(CompressionType::NONE, create_raw_cluster)]
    #[test_case(CompressionType::LZ4, create_lz4_cluster)]
    #[test_case(CompressionType::LZMA, create_lzma_cluster)]
    #[test_case(CompressionType::ZSTD, create_zstd_cluster)]
    fn test_cluster(comp: CompressionType, creator: ClusterCreator) {
        let reader = BufReader::new(creator(), End::None);
        let mut stream = reader.create_stream(Offset(0), End::None);
        let header = ClusterHeader::produce(stream.as_mut()).unwrap();
        assert_eq!(header.compression, comp);
        assert_eq!(header.offset_size, 1);
        assert_eq!(header.blob_count, Count(3));
        let cluster = Cluster::new(&reader).unwrap();
        assert_eq!(cluster.blob_count(), Count(3_u16));

        {
            let sub_reader = cluster.get_reader(Idx(0_u16)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream(Offset(0), End::None);
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let sub_reader = cluster.get_reader(Idx(1_u16)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream(Offset(0), End::None);
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let sub_reader = cluster.get_reader(Idx(2_u16)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream(Offset(0), End::None);
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }
}
