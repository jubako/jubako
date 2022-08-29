use crate::bases::*;
use crate::common::{PackHeader, PackHeaderInfo, PackKind};
use std::fmt::Debug;
use typenum::U31;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub struct DirectoryPackHeader {
    pub pack_header: PackHeader,
    pub entry_store_ptr_pos: Offset,
    pub key_store_ptr_pos: Offset,
    pub index_ptr_pos: Offset,
    pub entry_store_count: Count<u32>,
    pub index_count: Count<u32>,
    pub key_store_count: Count<u8>,
    pub free_data: FreeData<U31>,
}

impl DirectoryPackHeader {
    pub fn new(
        pack_info: PackHeaderInfo,
        free_data: FreeData<U31>,
        index_ptr_pos: Offset,
        index_count: Count<u32>,
        key_store_ptr_pos: Offset,
        key_store_count: Count<u8>,
        entry_store_ptr_pos: Offset,
        entry_store_count: Count<u32>,
    ) -> Self {
        DirectoryPackHeader {
            pack_header: PackHeader::new(PackKind::Directory, pack_info),
            index_ptr_pos,
            index_count,
            key_store_ptr_pos,
            key_store_count,
            entry_store_ptr_pos,
            entry_store_count,
            free_data,
        }
    }

    pub fn uuid(&self) -> Uuid {
        self.pack_header.uuid
    }
}

impl Producable for DirectoryPackHeader {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let pack_header = PackHeader::produce(stream)?;
        if pack_header.magic != PackKind::Directory {
            return Err(format_error!("Pack Magic is not DirectoryPack"));
        }
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

impl Writable for DirectoryPackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += self.pack_header.write(stream)?;
        written += self.index_ptr_pos.write(stream)?;
        written += self.entry_store_ptr_pos.write(stream)?;
        written += self.key_store_ptr_pos.write(stream)?;
        written += self.index_count.write(stream)?;
        written += self.entry_store_count.write(stream)?;
        written += self.key_store_count.write(stream)?;
        written += self.free_data.write(stream)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directorypackheader() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x64, // magic
            0x01, 0x00, 0x00, 0x00, // app_vendor_id
            0x01, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, // file_size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xee, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xdd, // index_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0x00, // entry_store_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xaa, // key_store_ptr_pos
            0x00, 0x00, 0x00, 0x50, // index count
            0x00, 0x00, 0x00, 0x60, // entry_store count
            0x05, //key_store count
        ];
        content.extend_from_slice(&[0xff; 31]);
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            DirectoryPackHeader::produce(stream.as_mut()).unwrap(),
            DirectoryPackHeader {
                pack_header: PackHeader {
                    magic: PackKind::Directory,
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
                free_data: FreeData::clone_from_slice(&[0xff; 31]),
            }
        );
    }
}
