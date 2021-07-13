use crate::bases::producing::*;
use crate::bases::reader::*;
use crate::bases::types::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompressionType {
    NONE = 0,
    LZ4 = 1,
    LZMA = 2,
    ZSTD = 3,
}

impl Producable for CompressionType {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        match producer.read_u8()? {
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
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let compression = CompressionType::produce(producer)?;
        let offset_size = producer.read_u8()?;
        let blob_count = Count::produce(producer)?;
        let cluster_size = Size::produce(producer)?;
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
        let mut producer = reader.create_stream(Offset::from(0), End::None);
        let header = ClusterHeader::produce(producer.as_mut())?;
        let data_size: Size = producer.read_sized(header.offset_size.into())?.into();
        let mut blob_offsets: Vec<Offset> = Vec::with_capacity((header.blob_count.0 + 1) as usize);
        unsafe { blob_offsets.set_len((header.blob_count.0).into()) }
        let mut first = true;
        for elem in blob_offsets.iter_mut() {
            if first {
                *elem = 0.into();
                first = false;
            } else {
                *elem = producer.read_sized(header.offset_size.into())?.into();
            }
            assert!(elem.is_valid(data_size));
        }
        blob_offsets.push(data_size.into());
        let raw_reader = reader.create_sub_reader(
            producer.tell_cursor(),
            End::Offset(header.cluster_size.into()),
        );
        let reader = match header.compression {
            CompressionType::NONE => {
                assert_eq!(
                    (producer.tell_cursor() + data_size).0,
                    header.cluster_size.0
                );
                raw_reader
            }
            CompressionType::LZ4 => {
                let producer = raw_reader.create_stream(Offset(0), End::None);
                Box::new(Lz4Reader::new(lz4::Decoder::new(producer)?, data_size))
            }
            CompressionType::LZMA => {
                let producer = raw_reader.create_stream(Offset(0), End::None);
                Box::new(LzmaReader::new(
                    lzma::LzmaReader::new_decompressor(producer)?,
                    data_size,
                ))
            }
            CompressionType::ZSTD => {
                let producer = raw_reader.create_stream(Offset(0), End::None);
                Box::new(ZstdReader::new(zstd::Decoder::new(producer)?, data_size))
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

    pub fn get_producer(&self, index: Idx<u16>) -> Result<Box<dyn Producer>> {
        let offset = self.blob_offsets[index.0 as usize];
        let end_offset = self.blob_offsets[(index.0 + 1) as usize];
        Ok(self.reader.create_stream(offset, End::Offset(end_offset)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_compressiontype() {
        let reader = BufReader::new(vec![0x00, 0x01, 0x02, 0x03, 0x4, 0xFF], End::None);
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            CompressionType::produce(producer.as_mut()).unwrap(),
            CompressionType::NONE
        );
        assert_eq!(
            CompressionType::produce(producer.as_mut()).unwrap(),
            CompressionType::LZ4
        );
        assert_eq!(
            CompressionType::produce(producer.as_mut()).unwrap(),
            CompressionType::LZMA
        );
        assert_eq!(
            CompressionType::produce(producer.as_mut()).unwrap(),
            CompressionType::ZSTD
        );
        assert_eq!(producer.tell_cursor(), Offset::from(4));
        assert!(CompressionType::produce(producer.as_mut()).is_err());
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
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            ClusterHeader::produce(producer.as_mut()).unwrap(),
            ClusterHeader {
                compression: CompressionType::NONE,
                offset_size: 1,
                blob_count: Count(2),
                cluster_size: Size(3)
            }
        );
    }

    #[test]
    fn test_cluster_raw() {
        let reader = BufReader::new(
            vec![
                0x00, // compression
                0x01, // offset_size
                0x00, 0x03, // blob_count
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1e, // cluster_size
                0x0f, // Data size
                0x05, // Offset of blob 1
                0x08, // Offset of blob 2
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of blob 0
                0x21, 0x22, 0x23, // Data of blob 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of blob 2
            ],
            End::None,
        );
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            ClusterHeader::produce(producer.as_mut()).unwrap(),
            ClusterHeader {
                compression: CompressionType::NONE,
                offset_size: 1,
                blob_count: Count(3),
                cluster_size: Size(30)
            }
        );
        let cluster = Cluster::new(&reader).unwrap();
        assert_eq!(cluster.blob_count(), Count(3_u16));

        {
            let mut sub_producer = cluster.get_producer(Idx(0_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(1_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(2_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }

    #[test]
    fn test_cluster_lz4() {
        let mut content = vec![
            0x01, // lz4 compression
            0x01, // offset_size
            0x00, 0x03, // blob_count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x31, // cluster_size
            0x0f, // Data size
            0x05, // Offset of blob 1
            0x08, // Offset of blob 2
        ];
        {
            let pos = content.len();
            let mut content_cursor = Cursor::new(&mut content);
            content_cursor.set_position(pos as u64);
            let mut encoder = lz4::EncoderBuilder::new()
                .level(16)
                .build(content_cursor)
                .unwrap();
            let mut blob_content = Cursor::new(vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of blob 0
                0x21, 0x22, 0x23, // Data of blob 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of blob 2
            ]);
            std::io::copy(&mut blob_content, &mut encoder).unwrap();
            encoder.finish().1.unwrap();
        }
        println!("content_len : {}\n", content.len());
        let reader = BufReader::new(content, End::None);
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            ClusterHeader::produce(producer.as_mut()).unwrap(),
            ClusterHeader {
                compression: CompressionType::LZ4,
                offset_size: 1,
                blob_count: Count(3),
                cluster_size: Size(49)
            }
        );
        let cluster = Cluster::new(&reader).unwrap();
        assert_eq!(cluster.blob_count(), Count(3_u16));

        {
            let mut sub_producer = cluster.get_producer(Idx(0_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(1_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(2_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }

    #[test]
    fn test_cluster_lzma() {
        let mut content = vec![
            0x02, // lzma compression
            0x01, // offset_size
            0x00, 0x03, // blob_count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x57, // cluster_size
            0x0f, // Data size
            0x05, // Offset of blob 1
            0x08, // Offset of blob 2
        ];
        {
            let pos = content.len();
            let mut content_cursor = Cursor::new(&mut content);
            content_cursor.set_position(pos as u64);
            let mut encoder = lzma::LzmaWriter::new_compressor(content_cursor, 9).unwrap();
            let mut blob_content = Cursor::new(vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of blob 0
                0x21, 0x22, 0x23, // Data of blob 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of blob 2
            ]);
            std::io::copy(&mut blob_content, &mut encoder).unwrap();
            encoder.finish().unwrap();
        }
        println!("content_len : {}\n", content.len());
        let reader = BufReader::new(content, End::None);
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            ClusterHeader::produce(producer.as_mut()).unwrap(),
            ClusterHeader {
                compression: CompressionType::LZMA,
                offset_size: 1,
                blob_count: Count(3),
                cluster_size: Size(87)
            }
        );
        let cluster = Cluster::new(&reader).unwrap();
        assert_eq!(cluster.blob_count(), Count(3_u16));

        {
            let mut sub_producer = cluster.get_producer(Idx(0_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(1_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(2_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }

    #[test]
    fn test_cluster_zstd() {
        let mut content = vec![
            0x03, // zstd compression
            0x01, // offset_size
            0x00, 0x03, // blob_count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x27, // cluster_size
            0x0f, // Data size
            0x05, // Offset of blob 1
            0x08, // Offset of blob 2
        ];
        {
            let pos = content.len();
            let mut content_cursor = Cursor::new(&mut content);
            content_cursor.set_position(pos as u64);
            let mut encoder = zstd::Encoder::new(content_cursor, 0).unwrap();
            let mut blob_content = Cursor::new(vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of blob 0
                0x21, 0x22, 0x23, // Data of blob 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of blob 2
            ]);
            std::io::copy(&mut blob_content, &mut encoder).unwrap();
            encoder.finish().unwrap();
        }
        println!("content_len : {}\n", content.len());
        let reader = BufReader::new(content, End::None);
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            ClusterHeader::produce(producer.as_mut()).unwrap(),
            ClusterHeader {
                compression: CompressionType::ZSTD,
                offset_size: 1,
                blob_count: Count(3),
                cluster_size: Size(39)
            }
        );
        let cluster = Cluster::new(&reader).unwrap();
        assert_eq!(cluster.blob_count(), Count(3_u16));

        {
            let mut sub_producer = cluster.get_producer(Idx(0_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(1_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(2_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }
}
