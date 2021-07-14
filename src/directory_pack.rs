mod key_store;
//mod index;

use crate::array_reader;
use crate::bases::*;
use crate::pack::*;
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::io::Read;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

struct FreeData47([u8; 47]);

impl PartialEq for FreeData47 {
    fn eq(&self, other: &Self) -> bool {
        self.0[..] == other.0[..]
    }
}
impl Debug for FreeData47 {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        Ok(())
    }
}
impl Deref for FreeData47 {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}
impl DerefMut for FreeData47 {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl FreeData47 {
    pub fn new() -> Self {
        Self([0; 47])
    }
}

#[derive(Debug, PartialEq)]
struct DirectoryPackHeader {
    pack_header: PackHeader,
    entry_store_ptr_pos: Offset,
    key_store_ptr_pos: Offset,
    index_ptr_pos: Offset,
    entry_store_count: Count<u32>,
    index_count: Count<u32>,
    key_store_count: Count<u8>,
    free_data: FreeData47,
}

impl Producable for DirectoryPackHeader {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let pack_header = PackHeader::produce(stream)?;
        let index_ptr_pos = Offset::produce(stream)?;
        let entry_store_ptr_pos = Offset::produce(stream)?;
        let key_store_ptr_pos = Offset::produce(stream)?;
        let index_count = Count::produce(stream)?;
        let entry_store_count = Count::produce(stream)?;
        let key_store_count = Count::produce(stream)?;
        let mut free_data = FreeData47::new();
        stream.read_exact(&mut free_data)?;
        Ok(DirectoryPackHeader {
            pack_header,
            entry_store_ptr_pos,
            key_store_ptr_pos,
            index_ptr_pos,
            entry_store_count,
            index_count,
            key_store_count,
            free_data,
        })
    }
}

#[derive(Debug)]
pub struct ContentAddress {
    pack_id: Idx<u8>,
    content_id: Idx<u32>,
}

impl Producable for ContentAddress {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let pack_id = stream.read_u8()?;
        let content_id = stream.read_sized(3)? as u32;
        Ok(ContentAddress {
            pack_id: pack_id.into(),
            content_id: content_id.into(),
        })
    }
}

pub struct DirectoryPack<'a> {
    header: DirectoryPackHeader,
    key_stores_ptrs: ArrayReader<'a, Offset, u8>,
    entry_stores_ptrs: ArrayReader<'a, Offset, u32>,
    index_ptrs: ArrayReader<'a, Offset, u32>,
    reader: Box<dyn Reader + 'a>,
    check_info: Cell<Option<CheckInfo>>,
}

impl<'a> DirectoryPack<'a> {
    pub fn new(reader: Box<dyn Reader>) -> Result<Self> {
        let mut stream = reader.create_stream(Offset(0), End::None);
        let header = DirectoryPackHeader::produce(stream.as_mut())?;
        let key_stores_ptrs = array_reader!(
            reader, at:header.key_store_ptr_pos, len:header.key_store_count, idx:u8 => (Offset, 8)
        );
        let entry_stores_ptrs = array_reader!(
            reader, at:header.entry_store_ptr_pos, len:header.entry_store_count, idx:u32 => (Offset, 8)
        );
        let index_ptrs = array_reader!(
            reader, at:header.index_ptr_pos, len:header.index_count, idx:u32 => (Offset, 8)
        );
        Ok(DirectoryPack {
            header,
            key_stores_ptrs,
            entry_stores_ptrs,
            index_ptrs,
            reader,
            check_info: Cell::new(None),
        })
    }
    pub fn get_free_data(&self) -> [u8; 47] {
        self.header.free_data.0
    }
}

impl Pack for DirectoryPack<'_> {
    fn kind(&self) -> PackKind {
        self.header.pack_header.magic
    }
    fn app_vendor_id(&self) -> u32 {
        self.header.pack_header.app_vendor_id
    }
    fn version(&self) -> (u8, u8) {
        (
            self.header.pack_header.major_version,
            self.header.pack_header.minor_version,
        )
    }
    fn uuid(&self) -> Uuid {
        self.header.pack_header.uuid
    }
    fn size(&self) -> Size {
        self.header.pack_header.file_size
    }
    fn check(&self) -> Result<bool> {
        if self.check_info.get().is_none() {
            let mut checkinfo_stream = self
                .reader
                .create_stream(self.header.pack_header.check_info_pos, End::None);
            let check_info = CheckInfo::produce(checkinfo_stream.as_mut())?;
            self.check_info.set(Some(check_info));
        }
        let mut check_stream = self.reader.create_stream(
            Offset::from(0),
            End::Offset(self.header.pack_header.check_info_pos),
        );
        self.check_info
            .get()
            .unwrap()
            .check(&mut check_stream.as_mut() as &mut dyn Read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directorypackheader() {
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
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xdd, // index_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0x00, // entry_store_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xaa, // key_store_ptr_pos
            0x00, 0x00, 0x00, 0x50, // index count
            0x00, 0x00, 0x00, 0x60, // entry_store count
            0x05, //key_store count
        ];
        content.extend_from_slice(&[0xff; 47]);
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            DirectoryPackHeader::produce(stream.as_mut()).unwrap(),
            DirectoryPackHeader {
                pack_header: PackHeader {
                    magic: PackKind::CONTENT,
                    app_vendor_id: 0x01000000_u32,
                    major_version: 0x01_u8,
                    minor_version: 0x02_u8,
                    uuid: Uuid::from_bytes([
                        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
                        0x0c, 0x0d, 0x0e, 0x0f
                    ]),
                    file_size: Size::from(0xffff_u64),
                    check_info_pos: Offset::from(0xffee_u64),
                },
                index_ptr_pos: Offset::from(0xeedd_u64),
                entry_store_ptr_pos: Offset::from(0xee00_u64),
                key_store_ptr_pos: Offset::from(0xeeaa_u64),
                index_count: Count::from(0x50_u32),
                entry_store_count: Count::from(0x60_u32),
                key_store_count: Count::from(0x05_u8),
                free_data: FreeData47([0xff; 47]),
            }
        );
    }
}
