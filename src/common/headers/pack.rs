use crate::bases::*;
use crate::common::{FullPackKind, PackKind};
use std::fmt::Debug;
use uuid::Uuid;

pub(crate) struct PackHeaderInfo {
    pub app_vendor_id: VendorId,
    pub file_size: Size,
    pub check_info_pos: Offset,
}

impl PackHeaderInfo {
    pub fn new(app_vendor_id: VendorId, file_size: Size, check_info_pos: Offset) -> Self {
        Self {
            app_vendor_id,
            file_size,
            check_info_pos,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PackHeader {
    pub magic: PackKind,
    pub app_vendor_id: VendorId,
    pub major_version: u8,
    pub minor_version: u8,
    pub uuid: Uuid,
    pub flags: u8,
    pub file_size: Size,
    pub check_info_pos: Offset,
}

impl PackHeader {
    pub fn new(magic: PackKind, pack_info: PackHeaderInfo) -> Self {
        PackHeader {
            magic,
            major_version: 0,
            minor_version: 2,
            uuid: Uuid::new_v4(),
            flags: 0,
            app_vendor_id: pack_info.app_vendor_id,
            file_size: pack_info.file_size,
            check_info_pos: pack_info.check_info_pos,
        }
    }

    pub fn check_info_size(&self) -> ASize {
        let check_info_size = self.file_size.into_u64()
            - Self::BLOCK_SIZE as u64
            - self.check_info_pos.into_u64()
            - BlockCheck::Crc32.size() as u64;
        ASize::new(check_info_size as usize)
    }
}

impl Parsable for PackHeader {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let magic = FullPackKind::parse(parser)?;
        let app_vendor_id = VendorId::parse(parser)?;
        let major_version = parser.read_u8()?;
        let minor_version = parser.read_u8()?;
        if (major_version, minor_version) != (0, 2) {
            return Err(Error::version_error(major_version, minor_version));
        }
        let uuid = Uuid::parse(parser)?;
        let flags = parser.read_u8()?;
        parser.skip(5)?;
        let file_size = Size::parse(parser)?;
        let check_info_pos = Offset::parse(parser)?;
        parser.skip(12)?;
        Ok(PackHeader {
            magic,
            app_vendor_id,
            major_version,
            minor_version,
            uuid,
            flags,
            file_size,
            check_info_pos,
        })
    }
}

impl BlockParsable for PackHeader {}

impl SizedParsable for PackHeader {
    const SIZE: usize = FullPackKind::SIZE
        + 4 // app_vendor_id
        + 1 // major
        + 1 // minor
        + Uuid::SIZE
        + 1 // flags
        + 5 // padding
        + Size::SIZE
        + Offset::SIZE
        + 12; // padding
}

impl Serializable for PackHeader {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += FullPackKind(self.magic).serialize(ser)?;
        written += self.app_vendor_id.serialize(ser)?;
        written += ser.write_u8(self.major_version)?;
        written += ser.write_u8(self.minor_version)?;
        written += self.uuid.serialize(ser)?;
        written += ser.write_u8(self.flags)?;
        written += ser.write_data(&[0_u8; 5])?;
        written += self.file_size.serialize(ser)?;
        written += self.check_info_pos.serialize(ser)?;
        written += ser.write_data(&[0_u8; 12])?;
        Ok(written)
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for PackHeader {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let name = match self.magic {
            PackKind::Manifest => "ManifestPack",
            PackKind::Container => "ContainerPack",
            PackKind::Content => "ContentPack",
            PackKind::Directory => "DirectoryPack",
        };
        let mut obj = serializer.serialize_struct(name, 4)?;
        obj.serialize_field("uuid", &self.uuid)?;
        obj.serialize_field("flags", &self.flags)?;
        obj.serialize_field("app_vendor_id", &self.app_vendor_id)?;
        obj.serialize_field("version", &(self.major_version, self.minor_version))?;
        obj.serialize_field("size", &self.file_size.into_u64())?;
        obj.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packheader() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x63, // magic
            0x00, 0x00, 0x00, 0x01, // app_vendor_id
            0x00, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, // flags
            0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // file_size
            0xee, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
        ];
        content.extend_from_slice(&[0xA1, 0x6A, 0x08, 0x3F]); // CRC
        let reader = Reader::from(content);
        let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero()).unwrap();
        assert_eq!(
            pack_header,
            PackHeader {
                magic: PackKind::Content,
                app_vendor_id: VendorId::from([00, 00, 00, 01]),
                major_version: 0x00_u8,
                minor_version: 0x02_u8,
                uuid: Uuid::from_bytes([
                    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f
                ]),
                flags: 0,
                file_size: Size::from(0xffff_u64),
                check_info_pos: Offset::from(0xffee_u64),
            }
        );
    }
}
