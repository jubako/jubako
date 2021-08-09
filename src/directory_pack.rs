mod entry;
mod entry_def;
mod index;
mod index_store;
mod key;
mod key_def;
mod key_store;

use self::index::{Index, IndexHeader};
use self::index_store::IndexStore;
use self::key_store::KeyStore;
use crate::bases::*;
use crate::pack::*;
use generic_array::typenum;
use std::cell::Cell;
use std::fmt::Debug;
use std::io::Read;
use uuid::Uuid;

#[derive(Debug, PartialEq)]
struct DirectoryPackHeader {
    pack_header: PackHeader,
    entry_store_ptr_pos: Offset,
    key_store_ptr_pos: Offset,
    index_ptr_pos: Offset,
    entry_store_count: Count<u32>,
    index_count: Count<u32>,
    key_store_count: Count<u8>,
    free_data: FreeData<typenum::U47>,
}

impl Producable for DirectoryPackHeader {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let pack_header = PackHeader::produce(stream)?;
        let index_ptr_pos = Offset::produce(stream)?;
        let entry_store_ptr_pos = Offset::produce(stream)?;
        let key_store_ptr_pos = Offset::produce(stream)?;
        let index_count = Count::<u32>::produce(stream)?;
        let entry_store_count = Count::<u32>::produce(stream)?;
        let key_store_count = Count::<u8>::produce(stream)?;
        let free_data = FreeData::produce(stream)?;
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

#[derive(Debug, PartialEq)]
pub struct ContentAddress {
    pack_id: Idx<u8>,
    content_id: Idx<u32>,
}

impl Producable for ContentAddress {
    type Output = Self;
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
    key_stores_ptrs: ArrayReader<'a, SizedOffset, u8>,
    entry_stores_ptrs: ArrayReader<'a, SizedOffset, u32>,
    index_ptrs: ArrayReader<'a, SizedOffset, u32>,
    reader: Box<dyn Reader + 'a>,
    check_info: Cell<Option<CheckInfo>>,
}

impl<'a> DirectoryPack<'a> {
    pub fn new(reader: Box<dyn Reader>) -> Result<Self> {
        let mut stream = reader.create_stream_all();
        let header = DirectoryPackHeader::produce(stream.as_mut())?;
        let key_stores_ptrs = ArrayReader::new_from_reader(
            reader.as_ref(),
            header.key_store_ptr_pos,
            header.key_store_count,
        );
        let entry_stores_ptrs = ArrayReader::new_from_reader(
            reader.as_ref(),
            header.entry_store_ptr_pos,
            header.entry_store_count,
        );
        let index_ptrs =
            ArrayReader::new_from_reader(reader.as_ref(), header.index_ptr_pos, header.index_count);
        Ok(DirectoryPack {
            header,
            key_stores_ptrs,
            entry_stores_ptrs,
            index_ptrs,
            reader,
            check_info: Cell::new(None),
        })
    }
    pub fn get_free_data(&self) -> &[u8] {
        self.header.free_data.as_ref()
    }

    pub fn get_index(&self, index_id: Idx<u32>) -> Result<Index> {
        let sized_offset: SizedOffset = self.index_ptrs.index(index_id);
        let mut index_stream = self.reader.create_stream_for(sized_offset);
        let index_header = IndexHeader::produce(index_stream.as_mut())?;
        let store = self.get_store(index_header.store_id)?;
        let index = Index::new(index_header, Box::new(store));
        Ok(index)
    }

    fn get_store(&self, store_id: Idx<u32>) -> Result<IndexStore> {
        let sized_offset = self.entry_stores_ptrs.index(store_id);
        IndexStore::new(self.reader.as_ref(), sized_offset)
    }

    pub fn get_key_store(&self, store_id: Idx<u8>) -> Result<KeyStore> {
        let sized_offset = self.key_stores_ptrs.index(store_id);
        KeyStore::new(self.reader.as_ref(), sized_offset)
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
                .create_stream_from(self.header.pack_header.check_info_pos);
            let check_info = CheckInfo::produce(checkinfo_stream.as_mut())?;
            self.check_info.set(Some(check_info));
        }
        let mut check_stream = self
            .reader
            .create_stream_to(End::Offset(self.header.pack_header.check_info_pos));
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
        let mut stream = reader.create_stream_all();
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
                free_data: FreeData::clone_from_slice(&[0xff; 47]),
            }
        );
    }
}
