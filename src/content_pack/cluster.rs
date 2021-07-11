use crate::bases::producing::*;
use crate::bases::types::*;
use crate::io::*;
use std::io::SeekFrom;

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
            _ => {
                producer.seek(SeekFrom::Current(-1)).unwrap();
                Err(Error::FormatError)
            }
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
    producer: Box<dyn Producer>,
}

impl Cluster {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let header = ClusterHeader::produce(producer)?;
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
        let producer = match header.compression {
            CompressionType::NONE => {
                assert_eq!(
                    (producer.tell_cursor() + data_size).0,
                    header.cluster_size.0
                );
                producer.sub_producer_at(
                    producer.tell_cursor(),
                    End::Offset(header.cluster_size.into()),
                )
            }
            CompressionType::LZ4 => {
                let raw_producer = producer.sub_producer_at(
                    producer.tell_cursor(),
                    End::Offset(header.cluster_size.into()),
                );
                Box::new(Lz4Wrapper::new(
                    lz4::Decoder::new(raw_producer).unwrap(),
                    data_size,
                ))
            }
            _ => {
                //[TODO] decompression from buf[read..header.cluster_size] to self.data
                Box::new(ProducerWrapper::<Vec<u8>>::new(
                    Vec::<u8>::with_capacity(data_size.0 as usize),
                    End::None,
                ))
            }
        };
        Ok(Cluster {
            blob_offsets,
            producer,
        })
    }

    pub fn blob_count(&self) -> Count<u16> {
        Count::from((self.blob_offsets.len() - 1) as u16)
    }

    pub fn get_producer(&self, index: Idx<u16>) -> Result<Box<dyn Producer>> {
        let offset = self.blob_offsets[index.0 as usize];
        let end_offset = self.blob_offsets[(index.0 + 1) as usize];
        Ok(self
            .producer
            .sub_producer_at(offset, End::Offset(end_offset)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Seek};

    #[test]
    fn test_compressiontype() {
        let mut producer =
            ProducerWrapper::<Vec<u8>>::new(vec![0x00, 0x01, 0x02, 0x03, 0x4, 0xFF], End::None);
        assert_eq!(
            CompressionType::produce(&mut producer).unwrap(),
            CompressionType::NONE
        );
        assert_eq!(
            CompressionType::produce(&mut producer).unwrap(),
            CompressionType::LZ4
        );
        assert_eq!(
            CompressionType::produce(&mut producer).unwrap(),
            CompressionType::LZMA
        );
        assert_eq!(
            CompressionType::produce(&mut producer).unwrap(),
            CompressionType::ZSTD
        );
        assert_eq!(producer.tell_cursor(), Offset::from(4));
        assert!(CompressionType::produce(&mut producer).is_err());
        assert_eq!(producer.tell_cursor(), Offset::from(4));
        assert!(CompressionType::produce(&mut producer).is_err());
    }

    #[test]
    fn test_clusterheader() {
        let mut producer = ProducerWrapper::<Vec<u8>>::new(
            vec![
                0x00, // compression
                0x01, // offset_size
                0x00, 0x02, // blob_count
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // cluster_size
            ],
            End::None,
        );
        assert_eq!(
            ClusterHeader::produce(&mut producer).unwrap(),
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
        let mut producer = ProducerWrapper::<Vec<u8>>::new(
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
        assert_eq!(
            ClusterHeader::produce(&mut producer).unwrap(),
            ClusterHeader {
                compression: CompressionType::NONE,
                offset_size: 1,
                blob_count: Count(3),
                cluster_size: Size(30)
            }
        );
        producer.seek(SeekFrom::Start(0)).unwrap();
        let cluster = Cluster::produce(&mut producer).unwrap();
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
        let mut producer = ProducerWrapper::<Vec<u8>>::new(content, End::None);
        assert_eq!(
            ClusterHeader::produce(&mut producer).unwrap(),
            ClusterHeader {
                compression: CompressionType::LZ4,
                offset_size: 1,
                blob_count: Count(3),
                cluster_size: Size(49)
            }
        );
        producer.seek(SeekFrom::Start(0)).unwrap();
        let cluster = Cluster::produce(&mut producer).unwrap();
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
