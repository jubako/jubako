use crate::bases::*;
use crate::pack::*;
use generic_array::typenum;

#[derive(Debug, PartialEq, Eq)]
pub struct MainPackHeader {
    pub pack_header: PackHeader,
    pub pack_count: Count<u8>,
    pub free_data: FreeData<typenum::U63>,
}

impl MainPackHeader {
    pub fn new(
        pack_info: PackHeaderInfo,
        free_data: FreeData<typenum::U63>,
        pack_count: Count<u8>,
    ) -> Self {
        MainPackHeader {
            pack_header: PackHeader::new(PackKind::Main, pack_info),
            pack_count,
            free_data,
        }
    }
}

impl SizedProducable for MainPackHeader {
    // PackHeader::Size (64) + Count<u8>::Size (1) + FreeData (63)
    type Size = typenum::U128;
}

impl Producable for MainPackHeader {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let pack_header = PackHeader::produce(stream)?;
        if pack_header.magic != PackKind::Main {
            return Err(format_error!("Pack Magic is not MainPack"));
        }
        let pack_count = Count::<u8>::produce(stream)?;
        let free_data = FreeData::produce(stream)?;
        Ok(Self {
            pack_header,
            pack_count,
            free_data,
        })
    }
}

impl Writable for MainPackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        self.pack_header.write(stream)?;
        self.pack_count.write(stream)?;
        self.free_data.write(stream)?;
        Ok(())
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
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            MainPackHeader::produce(stream.as_mut()).unwrap(),
            MainPackHeader {
                pack_header: PackHeader {
                    magic: PackKind::Main,
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
                pack_count: Count(2),
                free_data: FreeData::clone_from_slice(&[0xff; 63])
            }
        );
    }
}
