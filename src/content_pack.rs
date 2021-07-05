mod cluster;

use crate::bases::producing::*;
use crate::bases::types::*;
use crate::bases::*;
pub use cluster::Cluster;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::io::SeekFrom;
use std::ops::{Deref, DerefMut};

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

struct FreeData56([u8; 56]);

impl PartialEq for FreeData56 {
    fn eq(&self, other: &Self) -> bool {
        &self.0[..] == &other.0[..]
    }
}
impl Debug for FreeData56 {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        Ok(())
    }
}
impl Deref for FreeData56 {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}
impl DerefMut for FreeData56 {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl FreeData56 {
    pub fn new() -> Self {
        Self([0; 56])
    }
}

#[derive(Debug, PartialEq)]
struct ContentPackHeader {
    pack_header: PackHeader,
    entry_ptr_pos: Offset,
    cluster_ptr_pos: Offset,
    entry_count: Count<u32>,
    cluster_count: Count<u32>,
    free_data: FreeData56,
}

impl ContentPackHeader {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let pack_header = PackHeader::produce(producer)?;
        let entry_ptr_pos = Offset::produce(producer)?;
        let cluster_ptr_pos = Offset::produce(producer)?;
        let entry_count = Count::produce(producer)?;
        let cluster_count = Count::produce(producer)?;
        let mut free_data = FreeData56::new();
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

#[derive(Debug)]
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
        let mut cluster_producer = self
            .producer
            .borrow()
            .sub_producer_at(cluster_ptr, End::None);
        let cluster = Cluster::produce(cluster_producer.as_mut())?;
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
        self.header.free_data.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::*;

    #[test]
    fn test_contentpackheader() {
        let mut content = vec![
            0x61, 0x72, 0x78, 0x63, // magic
            0x01, 0x00, 0x00, 0x00, // app_vendor_id
            0x01, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uui
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, // file_size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xee, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0x00, // entry_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xdd, // cluster_ptr_pos
            0x00, 0x00, 0x00, 0x50, // entry ccount
            0x00, 0x00, 0x00, 0x60, // cluster ccount
        ];
        content.extend_from_slice(&[0xff; 56]);
        let mut producer = ProducerWrapper::<Vec<u8>>::new(content, End::None);
        assert_eq!(
            ContentPackHeader::produce(&mut producer).unwrap(),
            ContentPackHeader {
                pack_header: PackHeader {
                    _magic: 0x61727863_u32,
                    app_vendor_id: 0x01000000_u32,
                    major_version: 0x01_u8,
                    minor_version: 0x02_u8,
                    uuid: [
                        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
                        0x0c, 0x0d, 0x0e, 0x0f
                    ],
                    _file_size: Size::from(0xffff_u64),
                    _check_info_pos: Offset::from(0xffee_u64),
                },
                entry_ptr_pos: Offset::from(0xee00_u64),
                cluster_ptr_pos: Offset::from(0xeedd_u64),
                entry_count: Count::from(0x50_u32),
                cluster_count: Count::from(0x60_u32),
                free_data: FreeData56([0xff; 56]),
            }
        );
    }

    #[test]
    fn test_contentpack() {
        let mut content = vec![
            0x61, 0x72, 0x78, 0x63, // magic off:0
            0x01, 0x00, 0x00, 0x00, // app_vendor_id off:4
            0x01, // major_version off:8
            0x02, // minor_version off:9
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid off:10
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding off:26
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xB2, // file_size off:32
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xB2, // check_info_pos off:40
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, // entry_ptr_pos off:48
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8C, // cluster_ptr_pos off:56
            0x00, 0x00, 0x00, 0x03, // entry count off:64
            0x00, 0x00, 0x00, 0x01, // cluster count off:68
        ];
        content.extend_from_slice(&[0xff; 56]); // free_data off:72
        content.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00, // first entry info off:128
            0x00, 0x00, 0x00, 0x01, // second entry info off: 132
            0x00, 0x00, 0x00, 0x02, // third entry info off: 136
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x94, // first (and only) ptr pos. off:140
            // Cluster off:148
            0x00, // compression
            0x01, // offset_size
            0x00, 0x03, // blob_count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1e, // cluster_size
            0x0f, // Data size
            0x05, // Offset of blob 1
            0x08, // Offset of blob 2
            0x11, 0x12, 0x13, 0x14, 0x15, // Data of blob 0
            0x21, 0x22, 0x23, // Data of blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
            0x37, // Data of blob 2
                  // end 148+30 = 178
        ]);
        let producer = Box::new(ProducerWrapper::<Vec<u8>>::new(content, End::None));
        let content_pack = ContentPack::new(producer).unwrap();
        assert_eq!(content_pack.get_entry_count(), Count(3));
        assert_eq!(content_pack.get_app_vendor_id(), 0x01000000_u32);
        assert_eq!(content_pack.get_major_version(), 1);
        assert_eq!(content_pack.get_minor_version(), 2);
        assert_eq!(
            content_pack.get_uuid(),
            [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f
            ]
        );
        assert_eq!(&content_pack.get_free_data()[..], &[0xff; 56][..]);

        {
            let mut sub_producer = content_pack.get_content(Idx(0_u32)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let mut sub_producer = content_pack.get_content(Idx(1_u32)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let mut sub_producer = content_pack.get_content(Idx(2_u32)).unwrap();
            assert_eq!(sub_producer.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            sub_producer.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }

}
