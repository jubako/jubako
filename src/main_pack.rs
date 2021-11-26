use std::lazy::OnceCell;

use crate::bases::*;
use crate::pack::*;
use generic_array::typenum;
use std::cmp;
use std::io::{repeat, Read};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
struct MainPackHeader {
    pack_header: PackHeader,
    pack_count: Count<u8>,
    free_data: FreeData<typenum::U15>,
}

impl Producable for MainPackHeader {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let pack_header = PackHeader::produce(stream)?;
        let pack_count = Count::<u8>::produce(stream)?;
        let free_data = FreeData::produce(stream)?;
        Ok(Self {
            pack_header,
            pack_count,
            free_data,
        })
    }
}

#[derive(PartialEq, Debug)]
pub enum PackPos {
    Offset(Offset),
    Path(String),
}

#[derive(PartialEq, Debug)]
pub struct PackInfo {
    pub id: Uuid,
    pub pack_id: u8,
    pub free_data: FreeData<typenum::U103>,
    pub pack_size: Size,
    pub pack_check_info: Offset,
    pub pack_pos: PackPos,
}

impl Producable for PackInfo {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let id = Uuid::produce(stream)?;
        let pack_id = stream.read_u8()?;
        let free_data = FreeData::produce(stream)?;
        let pack_size = Size::produce(stream)?;
        let pack_check_info = Offset::produce(stream)?;
        let pack_offset = Offset::produce(stream)?;
        let pack_pos = if pack_offset.0 != 0 {
            stream.skip(Size(112))?;
            PackPos::Offset(pack_offset)
        } else {
            let v = PString::produce(stream)?;
            stream.skip(Size((111 - v.len()) as u64))?;
            let path = String::from_utf8(v)?;
            PackPos::Path(path)
        };
        Ok(Self {
            id,
            pack_id,
            free_data,
            pack_size,
            pack_check_info,
            pack_pos,
        })
    }
}

pub struct MainPack {
    header: MainPackHeader,
    reader: Box<dyn Reader>,
    directory_pack_info: PackInfo,
    pack_infos: Vec<PackInfo>,
    check_info: OnceCell<CheckInfo>,
}

impl MainPack {
    pub fn new(reader: Box<dyn Reader>) -> Result<Self> {
        let mut stream = reader.create_stream_all();
        let header = MainPackHeader::produce(stream.as_mut())?;
        let directory_pack_info = PackInfo::produce(stream.as_mut())?;
        let mut pack_infos: Vec<PackInfo> = Vec::with_capacity(header.pack_count.0 as usize);
        let mut max_id = 0;
        for _i in 0..header.pack_count.0 as u64 {
            let pack_info = PackInfo::produce(stream.as_mut())?;
            max_id = cmp::max(max_id, pack_info.pack_id);
            pack_infos.push(pack_info);
        }
        Ok(Self {
            header,
            reader,
            directory_pack_info,
            pack_infos,
            check_info: OnceCell::new(),
        })
    }
}

impl MainPack {
    pub fn pack_count(&self) -> u8 {
        self.header.pack_count.0
    }

    fn get_check_info<'b>(&'b self) -> Result<&'b CheckInfo> {
        self.check_info.get_or_try_init(|| self._get_check_info())
    }

    fn _get_check_info(&self) -> Result<CheckInfo> {
        let mut checkinfo_stream = self
            .reader
            .create_stream_from(self.header.pack_header.check_info_pos);
        CheckInfo::produce(checkinfo_stream.as_mut())
    }

    fn get_directory_pack_info(&self) -> &PackInfo {
        &self.directory_pack_info
    }

    fn get_content_pack_info(&self, pack_id: u8) -> Result<&PackInfo> {
        for pack_info in &self.pack_infos {
            if pack_info.pack_id == pack_id {
                return Ok(pack_info);
            }
        }
        Err(Error::Arg)
    }
}

struct CheckStream<'a> {
    source: &'a mut dyn Stream,
    start_safe_zone: u64,
}

impl<'a> CheckStream<'a> {
    pub fn new(source: &'a mut dyn Stream, pack_count: Count<u8>) -> Self {
        let start_safe_zone = 64 + 256 * (pack_count.0 as u64 + 1);
        Self {
            source,
            start_safe_zone,
        }
    }
}

impl Read for CheckStream<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Data we don't want to check are positionned between
        // 64 + k*256 + 144  and 64 + k*256 + 144 + 112
        // => between  (208 + k*256) and (320+ k*256)
        // for k < pack_count
        let offset = self.source.tell().0 as u64;
        if offset < 64 {
            let size = cmp::min(buf.len(), (64 - offset) as usize);
            self.source.read(&mut buf[..size])
        } else if offset >= self.start_safe_zone {
            self.source.read(buf)
        } else {
            let local_offset = ((offset - 64) % 256) as usize;
            if local_offset < 144 {
                let size = cmp::min(buf.len(), 144 - local_offset);
                self.source.read(&mut buf[..size])
            } else {
                let size = cmp::min(buf.len(), local_offset - 144);
                let size = repeat(0).read(&mut buf[..size])?;
                self.source.skip(Size::from(size)).unwrap();
                Ok(size)
            }
        }
    }
}

impl Pack for MainPack {
    fn kind(&self) -> PackKind {
        self.header.pack_header.magic
    }
    fn app_vendor_id(&self) -> u32 {
        self.header.pack_header.app_vendor_id
    }
    fn version(&self) -> (u8, u8) {
        (
            self.header.pack_header.major_version,
            self.header.pack_header.minor_version,
        )
    }
    fn uuid(&self) -> Uuid {
        self.header.pack_header.uuid
    }
    fn size(&self) -> Size {
        self.header.pack_header.file_size
    }
    fn check(&self) -> Result<bool> {
        let check_info = self.get_check_info()?;
        let mut check_stream = self
            .reader
            .create_stream_to(End::Offset(self.header.pack_header.check_info_pos));
        let mut check_stream = CheckStream::new(check_stream.as_mut(), self.header.pack_count);
        check_info.check(&mut check_stream as &mut dyn Read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::rc::Rc;

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
        content.extend_from_slice(&[0xff; 15]);
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
                free_data: FreeData::clone_from_slice(&[0xff; 15])
            }
        );
    }

    #[test]
    fn test_mainpack() {
        let mut rc_content = Rc::new(Vec::new());
        {
            let content = Rc::get_mut(&mut rc_content).unwrap();
            content.extend_from_slice(&[
                0x6a, 0x62, 0x6b, 0x6d, // magic
                0x01, 0x00, 0x00, 0x00, // app_vendor_id
                0x01, // major_version
                0x02, // minor_version
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f, // uuid
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x71, // file_size
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x50, // check_info_pos
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
                0x02, // pack_count
            ]);
            content.extend_from_slice(&[0xff; 15]);
            // First packInfo (directory pack)
            content.extend_from_slice(&[
                0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
                0x1e, 0x1f, // pack uuid
                0x00, //pack id
            ]);
            content.extend_from_slice(&[0xf0; 103]);
            content.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, // pack size
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, // pack check offset
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, // pack offset
            ]);
            content.extend_from_slice(&[0x00; 112]);
            // Second packInfo (first content pack)
            content.extend_from_slice(&[
                0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d,
                0x2e, 0x2f, // pack uuid
                0x01, //pack id
            ]);
            content.extend_from_slice(&[0xf1; 103]);
            content.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, // pack size
                0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, 0xff, // pack check offset
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, // pack offset
            ]);
            content.extend_from_slice(&[0x00; 112]);
            // Third packInfo (second content pack)
            content.extend_from_slice(&[
                0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d,
                0x3e, 0x3f, // pack uuid
                0x02, //pack id
            ]);
            content.extend_from_slice(&[0xf2; 103]);
            content.extend_from_slice(&[
                0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, // pack size
                0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, // pack check offset
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // pack offset
                8, b'p', b'a', b'c', b'k', b'p', b'a', b't', b'h',
            ]);
            content.extend_from_slice(&[0x00; 112 - 9]);
        }
        let hash = {
            let mut hasher = blake3::Hasher::new();
            let reader = BufReader::new_from_rc(Rc::clone(&rc_content), End::None);
            let mut stream = reader.create_stream_all();
            let mut check_stream = CheckStream::new(stream.as_mut(), Count(3));
            io::copy(&mut check_stream, &mut hasher).unwrap();
            hasher.finalize()
        };
        {
            let content = Rc::get_mut(&mut rc_content).unwrap();
            content.push(0x01);
            content.extend(hash.as_bytes());
        }
        let reader = Box::new(BufReader::new_from_rc(rc_content, End::None));
        let main_pack = MainPack::new(reader).unwrap();
        assert_eq!(main_pack.kind(), PackKind::Main);
        assert_eq!(main_pack.app_vendor_id(), 0x01000000);
        assert_eq!(main_pack.version(), (1, 2));
        assert_eq!(
            main_pack.uuid(),
            Uuid::from_bytes([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f
            ])
        );
        assert_eq!(main_pack.size(), Size::from(881_usize));
        assert!(main_pack.check().unwrap());
        assert_eq!(
            main_pack.get_directory_pack_info(),
            &PackInfo {
                id: Uuid::from_bytes([
                    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
                    0x1d, 0x1e, 0x1f
                ]),
                pack_id: 0,
                free_data: FreeData::clone_from_slice(&[0xf0; 103]),
                pack_size: Size(0xffff),
                pack_check_info: Offset(0xff),
                pack_pos: PackPos::Offset(Offset(0xff00))
            }
        );
        assert_eq!(
            main_pack.get_content_pack_info(1).unwrap(),
            &PackInfo {
                id: Uuid::from_bytes([
                    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c,
                    0x2d, 0x2e, 0x2f
                ]),
                pack_id: 1,
                free_data: FreeData::clone_from_slice(&[0xf1; 103]),
                pack_size: Size(0xffffff),
                pack_check_info: Offset(0xff00ff),
                pack_pos: PackPos::Offset(Offset(0xff00))
            }
        );
        assert_eq!(
            main_pack.get_content_pack_info(2).unwrap(),
            &PackInfo {
                id: Uuid::from_bytes([
                    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c,
                    0x3d, 0x3e, 0x3f
                ]),
                pack_id: 2,
                free_data: FreeData::clone_from_slice(&[0xf2; 103]),
                pack_size: Size(0xffffff),
                pack_check_info: Offset(0xffffff),
                pack_pos: PackPos::Path("packpath".to_string())
            }
        );
        assert!(main_pack.get_content_pack_info(3).is_err());
    }
}