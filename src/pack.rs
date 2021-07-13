use crate::bases::producing::*;
use crate::bases::types::*;
use std::fmt::Debug;
use std::io::{self, Read};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PackKind {
    ARX,
    DIRECTORY,
    CONTENT,
}

impl Producable for PackKind {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        match producer.read_u32()? {
            0x61_72_78_66_u32 => Ok(PackKind::ARX),
            0x61_72_78_69_u32 => Ok(PackKind::DIRECTORY),
            0x61_72_78_63_u32 => Ok(PackKind::CONTENT),
            _ => Err(Error::FormatError),
        }
    }
}

impl Producable for Uuid {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let mut v = [0_u8; 16];
        producer.read_exact(&mut v)?;
        Ok(Uuid::from_bytes(v))
    }
}

#[derive(Clone, Copy)]
enum CheckKind {
    NONE,
    BLAKE3,
}

impl Producable for CheckKind {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        match producer.read_u8()? {
            0_u8 => Ok(CheckKind::NONE),
            1_u8 => Ok(CheckKind::BLAKE3),
            _ => Err(Error::FormatError),
        }
    }
}

impl Producable for blake3::Hash {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let mut v = [0_u8; blake3::OUT_LEN];
        producer.read_exact(&mut v)?;
        Ok(v.into())
    }
}

#[derive(Clone, Copy)]
pub struct CheckInfo {
    b3hash: Option<blake3::Hash>,
}

impl Producable for CheckInfo {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let kind = CheckKind::produce(producer)?;
        let b3hash = match kind {
            CheckKind::BLAKE3 => Some(blake3::Hash::produce(producer)?),
            _ => None,
        };
        Ok(Self { b3hash })
    }
}

impl CheckInfo {
    pub fn check(&self, source: &mut dyn Read) -> Result<bool> {
        if let Some(b3hash) = self.b3hash {
            let mut hasher = blake3::Hasher::new();
            io::copy(source, &mut hasher)?;
            let hash = hasher.finalize();
            Ok(hash == b3hash)
        } else {
            Ok(true)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PackHeader {
    pub magic: PackKind,
    pub app_vendor_id: u32,
    pub major_version: u8,
    pub minor_version: u8,
    pub uuid: Uuid,
    pub file_size: Size,
    pub check_info_pos: Offset,
}

impl Producable for PackHeader {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let magic = PackKind::produce(producer)?;
        let app_vendor_id = producer.read_u32()?;
        let major_version = producer.read_u8()?;
        let minor_version = producer.read_u8()?;
        let uuid = Uuid::produce(producer)?;
        producer.skip(Size(6))?;
        let file_size = Size::produce(producer)?;
        let check_info_pos = Offset::produce(producer)?;
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

/// A Pack is the more global entity in Arx.
/// It is a "File", which can be a single file in the fs
/// or embedded in another file.
pub trait Pack {
    fn kind(&self) -> PackKind;
    fn app_vendor_id(&self) -> u32;
    fn version(&self) -> (u8, u8);
    fn uuid(&self) -> Uuid;
    fn size(&self) -> Size;
    fn check(&self) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bases::reader::*;

    #[test]
    fn test_packheader() {
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
            0x00, // No check
        ];
        content.extend_from_slice(&[0xff; 56]);
        let reader = BufReader::new(content, End::None);
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            PackHeader::produce(producer.as_mut()).unwrap(),
            PackHeader {
                magic: PackKind::CONTENT,
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