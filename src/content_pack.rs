mod cluster;

use crate::bases::producing::*;
use crate::bases::types::*;
use crate::bases::*;
pub use cluster::Cluster;
use std::cell::RefCell;
use std::io::SeekFrom;

#[derive(Debug, PartialEq)]
struct PackHeader {
    _magic: u32,
    app_vendor_id: u32,
    major_version: u8,
    minor_version: u8,
    uuid: [u8; 16],
    _file_size: Size,
    _check_info_pos: Offset,
}

impl PackHeader {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let magic = producer.read_u32()?;
        if magic != 0x61727863_u32 {
            return Err(Error::FormatError);
        }
        let app_vendor_id = producer.read_u32()?;
        let major_version = producer.read_u8()?;
        let minor_version = producer.read_u8()?;
        let mut uuid: [u8; 16] = [0_u8; 16];
        producer.read_exact(&mut uuid)?;
        producer.seek(SeekFrom::Current(6))?;
        let _file_size = Size::produce(producer)?;
        let _check_info_pos = Offset::produce(producer)?;
        Ok(PackHeader {
            _magic: magic,
            app_vendor_id,
            major_version,
            minor_version,
            uuid,
            _file_size,
            _check_info_pos,
        })
    }
}

struct ContentPackHeader {
    pack_header: PackHeader,
    entry_ptr_pos: Offset,
    cluster_ptr_pos: Offset,
    entry_count: Count<u32>,
    cluster_count: Count<u32>,
    free_data: [u8; 56],
}

impl ContentPackHeader {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let pack_header = PackHeader::produce(producer)?;
        let entry_ptr_pos = Offset::produce(producer)?;
        let cluster_ptr_pos = Offset::produce(producer)?;
        let entry_count = Count::produce(producer)?;
        let cluster_count = Count::produce(producer)?;
        let mut free_data: [u8; 56] = [0; 56];
        producer.read_exact(&mut free_data)?;
        Ok(ContentPackHeader {
            pack_header,
            entry_ptr_pos,
            cluster_ptr_pos,
            entry_count,
            cluster_count,
            free_data,
        })
    }
}

pub struct EntryInfo {
    cluster_index: Idx<u32>,
    blob_index: Idx<u16>,
}

impl Producable for EntryInfo {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let v = producer.read_u32()?;
        let blob_index = (v & 0xFFF) as u16;
        let cluster_index = v >> 12;
        Ok(EntryInfo {
            cluster_index: cluster_index.into(),
            blob_index: blob_index.into(),
        })
    }
}

pub struct ContentPack<'a> {
    header: ContentPackHeader,
    entry_infos: ArrayProducer<'a, EntryInfo, u32>,
    cluster_ptrs: ArrayProducer<'a, Offset, u32>,
    producer: RefCell<Box<dyn Producer + 'a>>,
}

impl<'a> ContentPack<'a> {
    pub fn new(mut producer: Box<dyn Producer>) -> Result<Self> {
        let header = ContentPackHeader::produce(producer.as_mut())?;
        let entry_infos = ArrayProducer::<EntryInfo, u32>::new(
            producer.sub_producer_at(
                header.entry_ptr_pos,
                End::Size(Size(header.entry_count.0 as u64 * 4)),
            ),
            header.entry_count,
            4,
        );
        let cluster_ptrs = ArrayProducer::<Offset, u32>::new(
            producer.sub_producer_at(
                header.cluster_ptr_pos,
                End::Size(Size(header.cluster_count.0 as u64 * 8)),
            ),
            header.cluster_count,
            8,
        );
        Ok(ContentPack {
            header,
            entry_infos,
            cluster_ptrs,
            producer: RefCell::new(producer),
        })
    }

    pub fn get_entry_count(&self) -> Count<u32> {
        self.header.entry_count
    }

    pub fn get_content(&self, index: Idx<u32>) -> Result<Box<dyn Producer + 'a>> {
        if !index.is_valid(self.header.entry_count) {
            return Err(Error::ArgError);
        }
        let entry_info = self.entry_infos.index(index);
        if !entry_info.cluster_index.is_valid(self.header.cluster_count) {
            return Err(Error::FormatError);
        }
        let cluster_ptr = self.cluster_ptrs.index(entry_info.cluster_index);
        self.producer
            .borrow_mut()
            .seek(SeekFrom::Start(cluster_ptr.0))?;
        let cluster = Cluster::produce(self.producer.borrow_mut().as_mut())?;
        cluster.get_producer(entry_info.blob_index)
    }

    pub fn get_app_vendor_id(&self) -> u32 {
        self.header.pack_header.app_vendor_id
    }
    pub fn get_major_version(&self) -> u8 {
        self.header.pack_header.major_version
    }
    pub fn get_minor_version(&self) -> u8 {
        self.header.pack_header.minor_version
    }
    pub fn get_uuid(&self) -> [u8; 16] {
        self.header.pack_header.uuid
    }
    pub fn get_free_data(&self) -> [u8; 56] {
        self.header.free_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Seek;

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
    fn test_cluster() {
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
        producer.seek(SeekFrom::Start(0));
        let cluster = Cluster::produce(&mut producer).unwrap();
        assert_eq!(cluster.blob_count(), Count(3_u16));

        {
            let mut sub_producer = cluster.get_producer(Idx(0_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v);
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(1_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v);
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let mut sub_producer = cluster.get_producer(Idx(2_u16)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v);
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }

}
