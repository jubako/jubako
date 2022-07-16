use crate::bases::*;
use crate::common::{PackHeader, PackHeaderInfo, PackKind};
use generic_array::typenum::U40;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
pub struct ContentPackHeader {
    pub pack_header: PackHeader,
    pub entry_ptr_pos: Offset,
    pub cluster_ptr_pos: Offset,
    pub entry_count: Count<u32>,
    pub cluster_count: Count<u32>,
    pub free_data: FreeData<U40>,
}

impl ContentPackHeader {
    pub fn new(
        pack_info: PackHeaderInfo,
        free_data: FreeData<U40>,
        cluster_ptr_pos: Offset,
        cluster_count: Count<u32>,
        entry_ptr_pos: Offset,
        entry_count: Count<u32>,
    ) -> Self {
        Self {
            pack_header: PackHeader::new(PackKind::Content, pack_info),
            entry_ptr_pos,
            cluster_ptr_pos,
            entry_count,
            cluster_count,
            free_data,
        }
    }
}

impl Producable for ContentPackHeader {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let pack_header = PackHeader::produce(stream)?;
        if pack_header.magic != PackKind::Content {
            return Err(format_error!("Pack Magic is not ContentPack"));
        }
        let entry_ptr_pos = Offset::produce(stream)?;
        let cluster_ptr_pos = Offset::produce(stream)?;
        let entry_count = Count::<u32>::produce(stream)?;
        let cluster_count = Count::<u32>::produce(stream)?;
        let free_data = FreeData::produce(stream)?;
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

impl Writable for ContentPackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        self.pack_header.write(stream)?;
        self.entry_ptr_pos.write(stream)?;
        self.cluster_ptr_pos.write(stream)?;
        self.entry_count.write(stream)?;
        self.cluster_count.write(stream)?;
        self.free_data.write(stream)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_contentpackheader() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x63, // magic
            0x01, 0x00, 0x00, 0x00, // app_vendor_id
            0x01, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uui
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, // file_size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xee, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xee, 0x00, // entry_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xdd, // cluster_ptr_pos
            0x00, 0x00, 0x00, 0x50, // entry ccount
            0x00, 0x00, 0x00, 0x60, // cluster ccount
        ];
        content.extend_from_slice(&[0xff; 40]);
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            ContentPackHeader::produce(stream.as_mut()).unwrap(),
            ContentPackHeader {
                pack_header: PackHeader {
                    magic: PackKind::Content,
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
                entry_ptr_pos: Offset::from(0xee00_u64),
                cluster_ptr_pos: Offset::from(0xeedd_u64),
                entry_count: Count::from(0x50_u32),
                cluster_count: Count::from(0x60_u32),
                free_data: FreeData::clone_from_slice(&[0xff; 40]),
            }
        );
    }
}
