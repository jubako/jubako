use crate::io::*;
use std::vec::Vec;
use std::rc::Rc;
use std::cell::RefCell;
use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum CompressionType {
    NONE = 0,
    LZ4 = 1,
    LZMA = 2,
    ZSTD = 3
}

impl CompressionType {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        match producer.read_u8()? {
            0 => Ok(CompressionType::NONE),
            1 => Ok(CompressionType::LZ4),
            2 => Ok(CompressionType::LZMA),
            3 => Ok(CompressionType::ZSTD),
            _ => Err(IOError{})
        }
    }
}

struct ClusterHeader {
    compression: CompressionType,
    offset_size: u8,
    blob_count: u16,
    cluster_size: u64
}

impl ClusterHeader {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let compression = CompressionType::produce(producer)?;
        let offset_size = producer.read_u8()?;
        let blob_count = producer.read_u16()?;
        let cluster_size = producer.read_u64()?;
        Ok(ClusterHeader {
            compression,
            offset_size,
            blob_count,
            cluster_size
        })
    }
}

struct Cluster {
    blob_offsets : Vec<u64>,
    producer: Box<dyn Producer>
}

impl Cluster {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let header = ClusterHeader::produce(producer)?;
        let data_size = producer.read_sized(header.offset_size.into())?;
        let mut blob_offsets = Vec::with_capacity(header.blob_count.into());
        unsafe { blob_offsets.set_len((header.blob_count-1).into()) }
        let mut first = true;
        for elem in blob_offsets.iter_mut() {
            if first {
                *elem = 0;
                first = false;
            } else {
                *elem = producer.read_sized(header.offset_size.into())?;
            }
            assert!(*elem <= data_size);
        }
        blob_offsets.push(data_size);
        let producer = match header.compression {
            CompressionType::NONE => {
                assert_eq!(producer.teel_cursor()+data_size, header.cluster_size);
                producer.sub_producer_at(producer.teel_cursor(), End::None)
            },
            _ => { //[TODO] decompression from buf[read..header.cluster_size] to self.data
                Box::new(BufferReader::new(Rc::new(Vec::<u8>::with_capacity(data_size as usize)), 0, End::Size(0)))
            }
        };
        Ok(Cluster {
            blob_offsets,
            producer
        })
    }

    fn get_producer(&self, index: Count) -> Result<Box<dyn Producer>> {
        let offset = self.blob_offsets[index.0 as usize];
        let end_offset = self.blob_offsets[(index.0+1) as usize];
        Ok(self.producer.sub_producer_at(offset, End::Offset(end_offset)))
    }
}

struct PackHeader {
    _magic:u32,
    app_vendor_id:u32,
    major_version:u8,
    minor_version:u8,
    uuid:[u8;16],
    _file_size:u64,
    _check_info_pos:u64
}

impl PackHeader {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let magic = producer.read_u32()?;
        if magic != 0x61727863_u32 {
            return Err(IOError{});
        }
        let app_vendor_id = producer.read_u32()?;
        let major_version = producer.read_u8()?;
        let minor_version = producer.read_u8()?;
        let mut uuid:[u8;16] = [0_u8;16];
        producer.read_data_into(16, &mut uuid)?;
        producer.move_cursor(6);
        let _file_size = producer.read_u64()?;
        let _check_info_pos = producer.read_u64()?;
        Ok(PackHeader {
            _magic: magic,
            app_vendor_id,
            major_version,
            minor_version,
            uuid,
            _file_size,
            _check_info_pos
        })
    }
}

struct ContentPackHeader {
    pack_header:PackHeader,
    entry_ptr_pos:u64,
    cluster_ptr_pos:u64,
    entry_count:u32,
    cluster_count:u32,
    free_data:[u8;56]
}

impl ContentPackHeader {
    pub fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let pack_header = PackHeader::produce(producer)?;
        let entry_ptr_pos = producer.read_u64()?;
        let cluster_ptr_pos = producer.read_u64()?;
        let entry_count = producer.read_u32()?;
        let cluster_count = producer.read_u32()?;
        let mut free_data:[u8;56] = [0; 56];
        producer.read_data_into(56, &mut free_data)?;
        Ok(ContentPackHeader {
            pack_header,
            entry_ptr_pos,
            cluster_ptr_pos,
            entry_count,
            cluster_count,
            free_data
        })
    }
}

pub struct EntryInfo {
    cluster_index: u32,
    blob_index: u16
}

impl Producable for EntryInfo {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let v = producer.read_u32()?;
        let blob_index = (v & 0xFFF) as u16;
        let cluster_index = v >> 12;
        Ok(EntryInfo{
            cluster_index,
            blob_index
        })
    }
}

pub struct ContentPack<'a> {
    header: ContentPackHeader,
    entry_infos: ArrayProducer<'a, EntryInfo>,
    cluster_ptrs: ArrayProducer<'a, u64>,
    producer: RefCell<Box<dyn Producer + 'a>>
}


impl<'a> ContentPack<'a> {
    pub fn new(mut producer: Box<dyn Producer>) -> Result<Self> {
        let header = ContentPackHeader::produce(producer.as_mut())?;
        let entry_infos = ArrayProducer::<EntryInfo>::new(
            producer.sub_producer_at(header.entry_ptr_pos, End::Size(header.entry_count as u64 *4)),
            Count(header.entry_count),
            4
        );
        let cluster_ptrs = ArrayProducer::<u64>::new(
            producer.sub_producer_at(header.cluster_ptr_pos, End::Size(header.cluster_count as u64 *8)),
            Count(header.cluster_count)
        );
        Ok(ContentPack {
            header,
            entry_infos,
            cluster_ptrs,
            producer: RefCell::new(producer)
        })
    }

    pub fn get_entry_count(&self) -> u32 {
        self.header.entry_count
    }

    pub fn get_content(&self, index: Count) -> Result<Box<dyn Producer + 'a>> {
        let entry_info = self.entry_infos.at(index)?;
        let cluster_ptr = self.cluster_ptrs.at(Count(entry_info.cluster_index))?;
        self.producer.borrow_mut().set_cursor(cluster_ptr);
        let cluster = Cluster::produce(self.producer.borrow_mut().as_mut())?;
        cluster.get_producer(Count(entry_info.blob_index.into()))
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
    pub fn get_uuid(&self) -> [u8;16] {
        self.header.pack_header.uuid
    }
    pub fn get_free_data(&self) -> [u8;56] {
        self.header.free_data
    }
}
