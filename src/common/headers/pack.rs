use crate::bases::*;
use crate::common::{FullPackKind, PackKind};
use std::fmt::Debug;
use uuid::Uuid;

pub struct PackHeaderInfo {
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
pub struct PackHeader {
    pub magic: PackKind,
    pub app_vendor_id: VendorId,
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
            minor_version: 1,
            uuid: Uuid::new_v4(),
            app_vendor_id: pack_info.app_vendor_id,
            file_size: pack_info.file_size,
            check_info_pos: pack_info.check_info_pos,
        }
    }
}

impl Producable for PackHeader {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let magic = FullPackKind::produce(flux)?;
        let app_vendor_id = VendorId::produce(flux)?;
        let major_version = flux.read_u8()?;
        let minor_version = flux.read_u8()?;
        let uuid = Uuid::produce(flux)?;
        flux.skip(Size::new(6))?;
        let file_size = Size::produce(flux)?;
        let check_info_pos = Offset::produce(flux)?;
        flux.skip(Size::new(16))?;
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

impl SizedProducable for PackHeader {
    const SIZE: usize = FullPackKind::SIZE
        + 4 // app_vendor_id
        + 1 // major
        + 1 // minor
        + Uuid::SIZE
        + 6 // padding
        + Size::SIZE
        + Offset::SIZE
        + 16; // padding
}

impl Writable for PackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += FullPackKind(self.magic).write(stream)?;
        written += self.app_vendor_id.write(stream)?;
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
            0x00, 0x00, 0x00, 0x01, // app_vendor_id
            0x01, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // file_size
            0xee, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // check_info_pos
            0x00, // No check
        ];
        content.extend_from_slice(&[0xff; 56]);
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        assert_eq!(
            PackHeader::produce(&mut flux).unwrap(),
            PackHeader {
                magic: PackKind::Content,
                app_vendor_id: VendorId::from([00, 00, 00, 01]),
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
