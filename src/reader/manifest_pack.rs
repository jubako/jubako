use std::sync::OnceLock;

use crate::bases::*;
use crate::common::{
    CheckInfo, ManifestCheckStream, ManifestPackHeader, Pack, PackHeader, PackInfo, PackKind,
};
use crate::reader::directory_pack::{ValueStore, ValueStoreTrait};
use std::cmp;
use uuid::Uuid;

pub struct PackOffsetsIter {
    offset: Offset,
    left: u16,
}

impl PackOffsetsIter {
    pub fn new(check_info_pos: Offset, pack_count: PackCount) -> Self {
        let offset = check_info_pos - pack_count * Size::from(PackInfo::BLOCK_SIZE);
        Self {
            offset,
            left: pack_count.into_u16(),
        }
    }
}

impl Iterator for PackOffsetsIter {
    type Item = Offset;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left != 0 {
            let offset = self.offset;
            self.offset += PackInfo::BLOCK_SIZE;
            self.left -= 1;
            Some(offset)
        } else {
            None
        }
    }
}

pub struct ManifestPack {
    pack_header: PackHeader,
    header: ManifestPackHeader,
    reader: Reader,
    directory_pack_info: PackInfo,
    pack_infos: Vec<PackInfo>,
    check_info: OnceLock<CheckInfo>,
    value_store: Option<ValueStore>,
    max_id: u16,
}

impl ManifestPack {
    pub fn new(reader: Reader) -> Result<Self> {
        let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero())?;
        if pack_header.magic != PackKind::Manifest {
            return Err(format_error!("Pack Magic is not ManifestPack"));
        }

        let header =
            reader.parse_block_at::<ManifestPackHeader>(Offset::from(PackHeader::BLOCK_SIZE))?;
        let pack_offsets = PackOffsetsIter::new(pack_header.check_info_pos, header.pack_count);
        let mut directory_pack_info = None;
        let mut pack_infos: Vec<PackInfo> = Vec::with_capacity(header.pack_count.into_usize());
        let mut max_id = 0;
        for pack_offset in pack_offsets {
            let pack_info = reader.parse_block_at::<PackInfo>(pack_offset)?;
            match pack_info.pack_kind {
                PackKind::Directory => directory_pack_info = Some(pack_info),
                _ => {
                    max_id = cmp::max(max_id, pack_info.pack_id.into_u16());
                    pack_infos.push(pack_info);
                }
            }
        }
        let vs_posinfo = header.value_store_posinfo;
        let value_store = if !vs_posinfo.is_zero() {
            Some(reader.parse_data_block::<ValueStore>(vs_posinfo)?)
        } else {
            None
        };
        Ok(Self {
            pack_header,
            header,
            reader,
            directory_pack_info: directory_pack_info.unwrap(),
            pack_infos,
            check_info: OnceLock::new(),
            value_store,
            max_id,
        })
    }

    fn packs_offset(&self) -> PackOffsetsIter {
        PackOffsetsIter::new(self.pack_header.check_info_pos, self.header.pack_count)
    }
}

impl ManifestPack {
    pub fn pack_count(&self) -> PackCount {
        self.header.pack_count
    }
    pub fn max_id(&self) -> u16 {
        self.max_id
    }

    fn get_check_info(&self) -> Result<&CheckInfo> {
        if self.check_info.get().is_none() {
            let _ = self.check_info.set(self._get_check_info()?);
        }
        Ok(self.check_info.get().unwrap())
    }

    fn _get_check_info(&self) -> Result<CheckInfo> {
        self.reader.parse_block_in::<CheckInfo>(
            self.pack_header.check_info_pos,
            self.pack_header.check_info_size(),
        )
    }

    pub fn get_pack_check_info(&self, uuid: Uuid) -> Result<CheckInfo> {
        let pack_info = if self.directory_pack_info.uuid == uuid {
            &self.directory_pack_info
        } else {
            self.get_content_pack_info_uuid(uuid)?
        };
        self.reader.parse_block_in::<CheckInfo>(
            pack_info.check_info_pos.offset,
            pack_info.check_info_pos.size,
        )
    }

    pub fn get_directory_pack_info(&self) -> &PackInfo {
        &self.directory_pack_info
    }

    pub fn get_content_pack_info(&self, pack_id: PackId) -> Result<&PackInfo> {
        for pack_info in &self.pack_infos {
            if pack_info.pack_id == pack_id {
                return Ok(pack_info);
            }
        }
        Err(Error::new_arg())
    }

    pub fn get_content_pack_info_uuid(&self, uuid: Uuid) -> Result<&PackInfo> {
        for pack_info in &self.pack_infos {
            if pack_info.uuid == uuid {
                return Ok(pack_info);
            }
        }
        Err(Error::new_arg())
    }

    pub fn get_pack_infos(&self) -> &[PackInfo] {
        &self.pack_infos
    }

    pub fn get_free_data(&self) -> PackFreeData {
        self.header.free_data
    }

    pub fn get_pack_free_data(&self, pack_id: PackId) -> Result<Option<&[u8]>> {
        let pack_info = if pack_id.into_u16() == 0 {
            &self.directory_pack_info
        } else {
            self.get_content_pack_info(pack_id)?
        };
        self.get_pack_free_data_raw(pack_info.free_data_id)
    }

    pub fn get_pack_free_data_uuid(&self, pack_uuid: Uuid) -> Result<Option<&[u8]>> {
        let pack_info = if self.directory_pack_info.uuid == pack_uuid {
            &self.directory_pack_info
        } else {
            self.get_content_pack_info_uuid(pack_uuid)?
        };
        self.get_pack_free_data_raw(pack_info.free_data_id)
    }

    pub fn get_pack_free_data_raw(&self, idx: ValueIdx) -> Result<Option<&[u8]>> {
        Ok(match &self.value_store {
            None => None,
            Some(v) => Some(v.get_data(idx, None)?),
        })
    }
}

impl Pack for ManifestPack {
    fn kind(&self) -> PackKind {
        self.pack_header.magic
    }
    fn app_vendor_id(&self) -> VendorId {
        self.pack_header.app_vendor_id
    }
    fn version(&self) -> (u8, u8) {
        (
            self.pack_header.major_version,
            self.pack_header.minor_version,
        )
    }
    fn uuid(&self) -> Uuid {
        self.pack_header.uuid
    }
    fn size(&self) -> Size {
        self.pack_header.file_size
    }
    fn check(&self) -> Result<bool> {
        let check_info = self.get_check_info()?;
        let mut check_stream = self.reader.create_stream(
            Offset::zero(),
            Size::from(self.pack_header.check_info_pos),
            false,
        )?;
        let mut check_stream =
            ManifestCheckStream::new_from_offset_iter(&mut check_stream, self.packs_offset());
        check_info.check(&mut check_stream)
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for ManifestPack {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut cont = serializer.serialize_struct("ManifestPack", 3)?;
        cont.serialize_field("uuid", &self.uuid())?;
        cont.serialize_field("directoryPack", &self.directory_pack_info)?;
        cont.serialize_field("contentPacks", &self.pack_infos)?;
        cont.end()
    }
}

#[cfg(feature = "explorable")]
impl Explorable for ManifestPack {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::Arc;

    #[test]
    fn test_mainpack() {
        let mut content = Arc::new(Vec::new());
        {
            let content = Arc::get_mut(&mut content).unwrap();

            // Pack header offset 0/0x00
            content.extend_from_slice(&[
                b'j', b'b', b'k', b'm', // magic
                0x00, 0x00, 0x00, 0x01, // app_vendor_id
                0x00, // major_version
                0x02, // minor_version
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f, // uuid
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
                0xE5, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // file_size
                0x80, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // check_info_pos
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
                0x00, 0x00, 0x00, 0x00, // reserved
            ]);
            content.extend_from_slice(&[0x1E, 0x59, 0x00, 0x9B]); // Crc32

            // Manifest pack heaader offset 64/0x40
            content.extend_from_slice(&[
                0x03, 0x00, // pack_count
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Value Store pos
            ]);
            content.extend_from_slice(&[0xff; 50]);
            content.extend_from_slice(&[0x77, 0x04, 0x2C, 0x88]); // Crc32

            // Offset 128/0x80

            // First packInfo (directory pack)
            content.extend_from_slice(&[
                0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
                0x1e, 0x1f, // pack uuid
                0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // pack size
                0x01, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, // pack check offset
                0x00, 0x00, //pack id
                b'd', // pack_kind
                0xF0, // pack_group
                0x01, 0x00, // free_data_id
            ]);
            // Offset 128 + 38 = 166/0xA6
            content.extend_from_slice(&[0x00; 214]); // empty pack_location
            content.extend_from_slice(&[0xD8, 0x88, 0xA4, 0x3C]); // Crc32

            // Offset 128 + 256 = 384/0xA6

            // Second packInfo (first content pack)
            content.extend_from_slice(&[
                0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d,
                0x2e, 0x2f, // pack uuid
                0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, // pack size
                0x21, 0x00, 0xff, 0x00, 0xff, 0x00, 0x00, 0x00, // pack check offset
                0x01, 0x00, //pack id
                b'c', //pack_kind
                0x00, // pack_group
                0x00, 0x00, // free_data_id
            ]);
            content.extend_from_slice(&[0x00; 214]); // empty pack_location
            content.extend_from_slice(&[0x89, 0xF9, 0x48, 0xD4]); // Crc32

            // Offset 384 + 256 = 640/x0280

            // Third packInfo (second content pack)
            content.extend_from_slice(&[
                0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d,
                0x3e, 0x3f, // pack uuid
                0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, // pack size
                0x01, 0x00, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, // pack check offset
                0x02, 0x00, //pack id,
                b'c', // pack_kind
                0x00, // pack_group
                0x00, 0x00, // free_data_id
                8, b'p', b'a', b'c', b'k', b'p', b'a', b't', b'h',
            ]);
            content.extend_from_slice(&[0x00; 214 - 9]);
            content.extend_from_slice(&[0x71, 0x0C, 0x8F, 0x11]); // Crc32
        }

        let hash = {
            let mut hasher = blake3::Hasher::new();
            let mut reader = std::io::Cursor::new(content.as_ref());
            let mut check_stream =
                ManifestCheckStream::new(&mut reader, Offset::new(128), PackCount::from(3));
            io::copy(&mut check_stream, &mut hasher).unwrap();
            hasher.finalize()
        };
        {
            let content = Arc::get_mut(&mut content).unwrap();
            // Check info Offset 640 + 256 = 896/0x0380 (check_info_pos)
            content.push(0x01);
            content.extend(hash.as_bytes());
            content.extend_from_slice(&[0x5D, 0xD6, 0x39, 0xD7]); // Crc32
        }

        // Footer 896 + 33 + 4 = 933/0x3A5
        let mut footer = [0; 64];
        footer.copy_from_slice(&content[..64]);
        footer.reverse();
        Arc::get_mut(&mut content)
            .unwrap()
            .extend_from_slice(&footer);

        // FileSize 933 + 64 = 993/0x03E5 (file_size)
        let content_size = content.size();
        let reader = Reader::new_from_arc(content, content_size);
        let main_pack = ManifestPack::new(reader).unwrap();
        assert_eq!(main_pack.kind(), PackKind::Manifest);
        assert_eq!(main_pack.app_vendor_id(), VendorId::from([00, 00, 00, 01]));
        assert_eq!(main_pack.version(), (0, 2));
        assert_eq!(
            main_pack.uuid(),
            Uuid::from_bytes([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f
            ])
        );
        assert_eq!(main_pack.size(), Size::new(997));
        assert!(main_pack.check().unwrap());
        assert_eq!(
            main_pack.get_directory_pack_info(),
            &PackInfo {
                uuid: Uuid::from_bytes([
                    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
                    0x1d, 0x1e, 0x1f
                ]),
                pack_id: PackId::from(0),
                pack_kind: PackKind::Directory,
                pack_group: 240,
                free_data_id: ValueIdx::from(1).into(),
                pack_size: Size::new(0xffff),
                check_info_pos: SizedOffset::new(0x01.into(), Offset::new(0xff)),
                pack_location: vec![],
            }
        );
        assert_eq!(
            main_pack.get_content_pack_info(PackId::from(1)).unwrap(),
            &PackInfo {
                uuid: Uuid::from_bytes([
                    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c,
                    0x2d, 0x2e, 0x2f
                ]),
                pack_id: PackId::from(1),
                pack_kind: PackKind::Content,
                pack_group: 0,
                free_data_id: ValueIdx::from(0).into(),
                pack_size: Size::new(0xffffff),
                check_info_pos: SizedOffset::new(0x21.into(), Offset::new(0xff00ff)),
                pack_location: vec![],
            }
        );
        assert_eq!(
            main_pack.get_content_pack_info(PackId::from(2)).unwrap(),
            &PackInfo {
                uuid: Uuid::from_bytes([
                    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c,
                    0x3d, 0x3e, 0x3f
                ]),
                pack_id: PackId::from(2),
                pack_kind: PackKind::Content,
                pack_group: 0,
                free_data_id: ValueIdx::from(0).into(),

                pack_size: Size::new(0xffffff),
                check_info_pos: SizedOffset::new(0x01.into(), Offset::new(0xffffff)),
                pack_location: vec![b'p', b'a', b'c', b'k', b'p', b'a', b't', b'h'],
            }
        );
        assert!(main_pack.get_content_pack_info(PackId::from(3)).is_err());
    }
}
