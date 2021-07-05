use crate::bases::producing::*;
use crate::bases::types::*;
use crate::bases::*;
use crate::io::*;
use std::cell::RefCell;
use std::io::SeekFrom;
use std::vec::Vec;

#[repr(u8)]
#[derive(Clone, Copy)]
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

struct Cluster {
    blob_offsets: Vec<Offset>,
    producer: Box<dyn Producer>,
}

impl Cluster {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let header = ClusterHeader::produce(producer)?;
        let data_size: Size = producer.read_sized(header.offset_size.into())?.into();
        let mut blob_offsets: Vec<Offset> = Vec::with_capacity(header.blob_count.0 as usize);
        unsafe { blob_offsets.set_len((header.blob_count.0 - 1).into()) }
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
                    (producer.teel_cursor() + data_size).0,
                    header.cluster_size.0
                );
                producer.sub_producer_at(producer.teel_cursor(), End::None)
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

    fn get_producer(&self, index: Idx<u16>) -> Result<Box<dyn Producer>> {
        let offset = self.blob_offsets[index.0 as usize];
        let end_offset = self.blob_offsets[(index.0 + 1) as usize];
        Ok(self
            .producer
            .sub_producer_at(offset, End::Offset(end_offset)))
    }
}

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
