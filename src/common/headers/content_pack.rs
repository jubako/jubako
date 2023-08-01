use crate::bases::*;
use crate::common::{PackHeader, PackHeaderInfo, PackKind};
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
pub struct ContentPackHeader {
    pub pack_header: PackHeader,
    pub content_ptr_pos: Offset,
    pub cluster_ptr_pos: Offset,
    pub content_count: ContentCount,
    pub cluster_count: ClusterCount,
    pub free_data: FreeData40,
}

impl ContentPackHeader {
    pub fn new(
        pack_info: PackHeaderInfo,
        free_data: FreeData40,
        cluster_ptr_pos: Offset,
        cluster_count: ClusterCount,
        content_ptr_pos: Offset,
        content_count: ContentCount,
    ) -> Self {
        Self {
            pack_header: PackHeader::new(PackKind::Content, pack_info),
            content_ptr_pos,
            cluster_ptr_pos,
            content_count,
            cluster_count,
            free_data,
        }
    }
}

impl Producable for ContentPackHeader {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let pack_header = PackHeader::produce(flux)?;
        if pack_header.magic != PackKind::Content {
            return Err(format_error!("Pack Magic is not ContentPack"));
        }
        let content_ptr_pos = Offset::produce(flux)?;
        let cluster_ptr_pos = Offset::produce(flux)?;
        let content_count = Count::<u32>::produce(flux)?.into();
        let cluster_count = Count::<u32>::produce(flux)?.into();
        let free_data = FreeData40::produce(flux)?;
        Ok(ContentPackHeader {
            pack_header,
            content_ptr_pos,
            cluster_ptr_pos,
            content_count,
            cluster_count,
            free_data,
        })
    }
}

impl Writable for ContentPackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += self.pack_header.write(stream)?;
        written += self.content_ptr_pos.write(stream)?;
        written += self.cluster_ptr_pos.write(stream)?;
        written += self.content_count.write(stream)?;
        written += self.cluster_count.write(stream)?;
        written += self.free_data.write(stream)?;
        Ok(written)
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
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        assert_eq!(
            ContentPackHeader::produce(&mut flux).unwrap(),
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
                content_ptr_pos: Offset::from(0xee00_u64),
                cluster_ptr_pos: Offset::from(0xeedd_u64),
                content_count: ContentCount::from(0x50_u32),
                cluster_count: ClusterCount::from(0x60_u32),
                free_data: [0xff; 40],
            }
        );
    }
}
