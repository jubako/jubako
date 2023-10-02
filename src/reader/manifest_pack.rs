use std::cell::OnceCell;

use crate::bases::*;
use crate::common::{CheckInfo, ManifestCheckStream, ManifestPackHeader, Pack, PackInfo, PackKind};
use crate::reader::directory_pack::{ValueStore, ValueStoreTrait};
use std::cmp;
use std::io::Read;

use uuid::Uuid;

pub struct ManifestPack {
    header: ManifestPackHeader,
    reader: Reader,
    directory_pack_info: PackInfo,
    pack_infos: Vec<PackInfo>,
    check_info: OnceCell<CheckInfo>,
    value_store: Option<ValueStore>,
    max_id: u8,
}

impl ManifestPack {
    pub fn new(reader: Reader) -> Result<Self> {
        let mut flux = reader.create_flux_all();
        let header = ManifestPackHeader::produce(&mut flux)?;
        flux.seek(header.packs_offset());
        let mut directory_pack_info = None;
        let mut pack_infos: Vec<PackInfo> = Vec::with_capacity(header.pack_count.into_usize());
        let mut max_id = 0;
        for _i in header.pack_count {
            let pack_info = PackInfo::produce(&mut flux)?;
            match pack_info.pack_kind {
                PackKind::Directory => directory_pack_info = Some(pack_info),
                _ => {
                    max_id = cmp::max(max_id, pack_info.pack_id.into_u8());
                    pack_infos.push(pack_info);
                }
            }
        }
        let vs_posinfo = header.value_store_posinfo;
        let value_store = if !vs_posinfo.is_zero() {
            Some(ValueStore::new(&reader, vs_posinfo)?)
        } else {
            None
        };
        Ok(Self {
            header,
            reader,
            directory_pack_info: directory_pack_info.unwrap(),
            pack_infos,
            check_info: OnceCell::new(),
            value_store,
            max_id,
        })
    }
}

impl ManifestPack {
    pub fn pack_count(&self) -> PackCount {
        self.header.pack_count
    }
    pub fn max_id(&self) -> u8 {
        self.max_id
    }

    fn get_check_info(&self) -> Result<&CheckInfo> {
        self.check_info.get_or_try_init(|| self._get_check_info())
    }

    fn _get_check_info(&self) -> Result<CheckInfo> {
        let mut checkinfo_flux = self
            .reader
            .create_flux_from(self.header.pack_header.check_info_pos);
        CheckInfo::produce(&mut checkinfo_flux)
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

    pub fn get_free_data(&self, pack_id: PackId) -> Result<Option<&[u8]>> {
        let pack_info = if pack_id.into_u8() == 0 {
            &self.directory_pack_info
        } else {
            self.get_content_pack_info(pack_id)?
        };
        Ok(match &self.value_store {
            None => None,
            Some(v) => Some(v.get_data(pack_info.free_data_id, None)?),
        })
    }
}

impl Pack for ManifestPack {
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
        let mut check_flux = self
            .reader
            .create_flux_to(End::Offset(self.header.pack_header.check_info_pos));
        let mut check_stream = ManifestCheckStream::new(
            &mut check_flux,
            self.header.packs_offset(),
            self.header.pack_count,
        );
        check_info.check(&mut check_stream as &mut dyn Read)
    }
}

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
            content.extend_from_slice(&[
                b'j', b'b', b'k', b'm', // magic
                0x01, 0x00, 0x00, 0x00, // app_vendor_id
                0x01, // major_version
                0x02, // minor_version
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f, // uuid
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x71, // file_size
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x80, // check_info_pos
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
                0x03, // pack_count
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Value Store pos
            ]);
            content.extend_from_slice(&[0xff; 55]);

            // Offset 128
            // First packInfo (directory pack)
            content.extend_from_slice(&[
                0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
                0x1e, 0x1f, // pack uuid
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, // pack size
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, // pack check offset
                b'd', // pack_kind
                0x00, //pack id
                0x00, // pack_group
                0x00, // padding
                0x00, 0x00, // free_data_id
            ]);
            // Offset 128 + 38 = 166
            content.extend_from_slice(&[0x00; 218]); // empty pack_location

            // Offset 128 + 256 = 382

            // Second packInfo (first content pack)
            content.extend_from_slice(&[
                0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d,
                0x2e, 0x2f, // pack uuid
                0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, // pack size
                0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, 0xff, // pack check offset
                b'c', //pack_kind
                0x01, //pack id
                0x00, // pack_group
                0x00, // padding
                0x00, 0x00, // free_data_id
            ]);
            content.extend_from_slice(&[0x00; 218]); // empty pack_location

            // Offset 382 + 256 = 638

            // Third packInfo (second content pack)
            content.extend_from_slice(&[
                0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d,
                0x3e, 0x3f, // pack uuid
                0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, // pack size
                0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, // pack check offset
                b'c', // pack_kind
                0x02, //pack id
                0x00, // pack_group
                0x00, // padding
                0x00, 0x00, // free_data_id
                8, b'p', b'a', b'c', b'k', b'p', b'a', b't', b'h',
            ]);
            content.extend_from_slice(&[0x00; 218 - 9]);

            // Offset 638 + 256 = 894
        }
        let hash = {
            let mut hasher = blake3::Hasher::new();
            let reader = Reader::new_from_arc(Arc::clone(&content) as Arc<dyn Source>, End::None);
            let mut flux = reader.create_flux_all();
            let mut check_stream =
                ManifestCheckStream::new(&mut flux, Offset::new(128), PackCount::from(3));
            io::copy(&mut check_stream, &mut hasher).unwrap();
            hasher.finalize()
        };
        {
            let content = Arc::get_mut(&mut content).unwrap();
            content.push(0x01);
            content.extend(hash.as_bytes());
        }
        let reader = Reader::new_from_arc(content, End::None);
        let main_pack = ManifestPack::new(reader).unwrap();
        assert_eq!(main_pack.kind(), PackKind::Manifest);
        assert_eq!(main_pack.app_vendor_id(), 0x01000000);
        assert_eq!(main_pack.version(), (1, 2));
        assert_eq!(
            main_pack.uuid(),
            Uuid::from_bytes([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f
            ])
        );
        assert_eq!(main_pack.size(), Size::new(881));
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
                pack_group: 0,
                free_data_id: ValueIdx::from(0).into(),
                pack_size: Size::new(0xffff),
                check_info_pos: Offset::new(0xff),
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
                check_info_pos: Offset::new(0xff00ff),
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
                check_info_pos: Offset::new(0xffffff),
                pack_location: vec![b'p', b'a', b'c', b'k', b'p', b'a', b't', b'h'],
            }
        );
        assert!(main_pack.get_content_pack_info(PackId::from(3)).is_err());
    }
}
