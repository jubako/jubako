use crate::bases::*;
use crate::common::PackKind;
use std::fmt::Debug;
use uuid::Uuid;

pub struct PackHeaderInfo {
    pub app_vendor_id: u32,
    pub file_size: Size,
    pub check_info_pos: Offset,
}

impl PackHeaderInfo {
    pub fn new(app_vendor_id: u32, file_size: Size, check_info_pos: Offset) -> Self {
        Self {
            app_vendor_id,
            file_size,
            check_info_pos,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PackHeader {
    pub magic: PackKind,
    pub app_vendor_id: u32,
    pub major_version: u8,
    pub minor_version: u8,
    pub uuid: Uuid,
    pub file_size: Size,
    pub check_info_pos: Offset,
}

impl PackHeader {
    pub fn new(magic: PackKind, pack_info: PackHeaderInfo) -> Self {
        PackHeader {
            magic,
            major_version: 0,
            minor_version: 0,
            uuid: Uuid::new_v4(),
            app_vendor_id: pack_info.app_vendor_id,
            file_size: pack_info.file_size,
            check_info_pos: pack_info.check_info_pos,
        }
    }
}

impl Producable for PackHeader {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let magic = PackKind::produce(stream)?;
        let app_vendor_id = stream.read_u32()?;
        let major_version = stream.read_u8()?;
        let minor_version = stream.read_u8()?;
        let uuid = Uuid::produce(stream)?;
        stream.skip(Size::new(6))?;
        let file_size = Size::produce(stream)?;
        let check_info_pos = Offset::produce(stream)?;
        stream.skip(Size::new(16))?;
        Ok(PackHeader {
            magic,
            app_vendor_id,
            major_version,
            minor_version,
            uuid,
            file_size,
            check_info_pos,
        })
    }
}

impl Writable for PackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += self.magic.write(stream)?;
        written += stream.write_u32(self.app_vendor_id)?;
        written += stream.write_u8(self.major_version)?;
        written += stream.write_u8(self.minor_version)?;
        written += self.uuid.write(stream)?;
        written += stream.write_data(&[0_u8; 6])?;
        written += self.file_size.write(stream)?;
        written += self.check_info_pos.write(stream)?;
        written += stream.write_data(&[0_u8; 16])?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packheader() {
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
            0x00, // No check
        ];
        content.extend_from_slice(&[0xff; 56]);
        let mut stream = Stream::from(content);
        assert_eq!(
            PackHeader::produce(&mut stream).unwrap(),
            PackHeader {
                magic: PackKind::Content,
                app_vendor_id: 0x01000000_u32,
                major_version: 0x01_u8,
                minor_version: 0x02_u8,
                uuid: Uuid::from_bytes([
                    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f
                ]),
                file_size: Size::from(0xffff_u64),
                check_info_pos: Offset::from(0xffee_u64),
            }
        );
    }
}
