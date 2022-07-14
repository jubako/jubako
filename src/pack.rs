use crate::bases::*;
use std::fmt::Debug;
use std::io::{self, Read};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PackKind {
    Main,
    Directory,
    Content,
}

impl Producable for PackKind {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        match stream.read_u32()? {
            0x6a_62_6b_6d_u32 => Ok(PackKind::Main),      // jbkm
            0x6a_62_6b_64_u32 => Ok(PackKind::Directory), // jbkd
            0x6a_62_6b_63_u32 => Ok(PackKind::Content),   // jbkc
            _ => Err(format_error!("Invalid pack kind", stream)),
        }
    }
}

impl Writable for PackKind {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        match self {
            PackKind::Main => stream.write_u32(0x6a_62_6b_6d_u32),
            PackKind::Directory => stream.write_u32(0x6a_62_6b_64_u32),
            PackKind::Content => stream.write_u32(0x6a_62_6b_63_u32),
        }
    }
}

impl Producable for Uuid {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let mut v = [0_u8; 16];
        stream.read_exact(&mut v)?;
        Ok(Uuid::from_bytes(v))
    }
}
impl Writable for Uuid {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        stream.write_all(self.as_bytes())?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum CheckKind {
    None = 0,
    Blake3 = 1,
}

impl Producable for CheckKind {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let kind = stream.read_u8()?;
        match kind {
            0_u8 => Ok(CheckKind::None),
            1_u8 => Ok(CheckKind::Blake3),
            _ => Err(format_error!(
                &format!("Invalid check kind {}", kind),
                stream
            )),
        }
    }
}

impl Writable for CheckKind {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        stream.write_u8(*self as u8)?;
        Ok(())
    }
}

impl Producable for blake3::Hash {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let mut v = [0_u8; blake3::OUT_LEN];
        stream.read_exact(&mut v)?;
        Ok(v.into())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CheckInfo {
    b3hash: Option<blake3::Hash>,
}

impl Producable for CheckInfo {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let kind = CheckKind::produce(stream)?;
        let b3hash = match kind {
            CheckKind::Blake3 => Some(blake3::Hash::produce(stream)?),
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
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let magic = PackKind::produce(stream)?;
        let app_vendor_id = stream.read_u32()?;
        let major_version = stream.read_u8()?;
        let minor_version = stream.read_u8()?;
        let uuid = Uuid::produce(stream)?;
        stream.skip(Size(6))?;
        let file_size = Size::produce(stream)?;
        let check_info_pos = Offset::produce(stream)?;
        stream.skip(Size(16))?;
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
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        self.magic.write(stream)?;
        stream.write_u32(self.app_vendor_id)?;
        stream.write_u8(self.major_version)?;
        stream.write_u8(self.minor_version)?;
        self.uuid.write(stream)?;
        stream.write_all(&[0_u8; 6])?;
        self.file_size.write(stream)?;
        self.check_info_pos.write(stream)?;
        stream.write_all(&[0_u8; 16])?;
        Ok(())
    }
}

/// A Pack is the more global entity in Jubako.
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
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            PackHeader::produce(stream.as_mut()).unwrap(),
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
