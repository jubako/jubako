use crate::bases::*;
use crate::common::{PackHeader, PackHeaderInfo, PackKind};
use generic_array::typenum::U128;

#[derive(Debug, PartialEq, Eq)]
pub struct ManifestPackHeader {
    pub pack_header: PackHeader,
    pub pack_count: PackCount,
    pub free_data: FreeData63,
}

impl ManifestPackHeader {
    pub fn new(pack_info: PackHeaderInfo, free_data: FreeData63, pack_count: PackCount) -> Self {
        ManifestPackHeader {
            pack_header: PackHeader::new(PackKind::Manifest, pack_info),
            pack_count,
            free_data,
        }
    }
}

impl SizedProducable for ManifestPackHeader {
    // PackHeader::Size (64) + PackCount::Size (1) + FreeData (63)
    type Size = U128;
}

impl Producable for ManifestPackHeader {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let pack_header = PackHeader::produce(stream)?;
        if pack_header.magic != PackKind::Manifest {
            return Err(format_error!("Pack Magic is not ManifestPack"));
        }
        let pack_count = Count::<u8>::produce(stream)?.into();
        let free_data = FreeData63::produce(stream)?;
        Ok(Self {
            pack_header,
            pack_count,
            free_data,
        })
    }
}

impl Writable for ManifestPackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += self.pack_header.write(stream)?;
        written += self.pack_count.write(stream)?;
        written += self.free_data.write(stream)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_mainpackheader() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x6d, // magic
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
            0x02, // pack_count
        ];
        content.extend_from_slice(&[0xff; 63]);
        let reader = Reader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            ManifestPackHeader::produce(&mut stream).unwrap(),
            ManifestPackHeader {
                pack_header: PackHeader {
                    magic: PackKind::Manifest,
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
                pack_count: PackCount::from(2),
                free_data: FreeData63::clone_from_slice(&[0xff; 63])
            }
        );
    }
}
