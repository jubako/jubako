use super::{ContentPack, DirectoryPack, ManifestPack, PackLocatorTrait};
use crate::bases::*;
use crate::common::{ContainerPackHeader, FullPackKind, Pack, PackKind, PackLocator};
use std::collections::HashMap;
use uuid::Uuid;

pub struct ContainerPack {
    packs_uuid: Vec<Uuid>,
    packs: HashMap<Uuid, Reader>,
}

fn packs_offset(header: &ContainerPackHeader) -> Offset {
    (header.file_size
        - Size::new(ContainerPackHeader::SIZE as u64)
        - Size::new(header.pack_count.into_u64() * PackLocator::SIZE as u64))
    .into()
}

impl ContainerPack {
    pub fn new(reader: Reader) -> Result<Self> {
        let mut flux = reader.create_flux_all();
        let header = ContainerPackHeader::produce(&mut flux)?;
        flux.seek(packs_offset(&header));
        let mut packs_uuid = Vec::with_capacity(header.pack_count.into_usize());
        let mut packs = HashMap::with_capacity(header.pack_count.into_usize());
        for _idx in header.pack_count {
            let pack_locator = PackLocator::produce(&mut flux)?;
            let pack_reader = reader
                .create_sub_reader(pack_locator.pack_pos, End::Size(pack_locator.pack_size))
                .into();
            packs.insert(pack_locator.uuid, pack_reader);
            packs_uuid.push(pack_locator.uuid);
        }
        Ok(Self { packs_uuid, packs })
    }

    pub fn new_fake(reader: Reader, uuid: Uuid) -> Self {
        let header = ContainerPackHeader::new(1.into(), reader.size());
        let mut packs_uuid = Vec::with_capacity(header.pack_count.into_usize());
        let mut packs = HashMap::with_capacity(header.pack_count.into_usize());
        packs.insert(uuid, reader);
        packs_uuid.push(uuid);
        Self { packs_uuid, packs }
    }

    pub fn pack_count(&self) -> PackCount {
        (self.packs.len() as u8).into()
    }

    pub fn get_pack_uuid(&self, idx: PackId) -> uuid::Uuid {
        self.packs_uuid[idx.into_usize()]
    }

    pub fn get_pack_reader_from_idx(&self, idx: PackId) -> Option<Reader> {
        self.get_pack_reader(&self.get_pack_uuid(idx))
    }

    pub fn get_pack_reader(&self, uuid: &Uuid) -> Option<Reader> {
        self.packs.get(uuid).cloned()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a uuid::Uuid, &'a Reader)> + 'a {
        self.packs.iter()
    }

    pub fn check(&self) -> Result<bool> {
        for reader in self.packs.values() {
            let pack_kind = FullPackKind::produce(&mut reader.create_flux_all())?;
            let ok = match pack_kind {
                PackKind::Manifest => ManifestPack::new(reader.clone())?.check()?,
                PackKind::Directory => DirectoryPack::new(reader.clone())?.check()?,
                PackKind::Content => ContentPack::new(reader.clone())?.check()?,
                PackKind::Container => todo!(),
            };
            if !ok {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl PackLocatorTrait for ContainerPack {
    fn locate(&self, uuid: Uuid, _path: &[u8]) -> Result<Option<Reader>> {
        Ok(self.get_pack_reader(&uuid))
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{PackPos, PackKind};
    use std::io;
    use std::sync::Arc;

    #[test]
    fn test_mainpack() {
        let mut content = Arc::new(Vec::new());
        {
            let content = Arc::get_mut(&mut content).unwrap();
            content.extend_from_slice(&[
                0x6a, 0x62, 0x6b, 0x6d, // magic
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
                0x02, // pack_count
            ]);
            content.extend_from_slice(&[0xff; 63]);

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
            let reader = Reader::new_from_arc(Arc::clone(&content) as Arc<dyn Source>, End::None);
            let mut check_stream = CheckStream::new((&reader).into(), PackCount::from(3));
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
        assert!(main_pack.check_manifest_only().unwrap());
        // The pack offset are random. So check of embedded packs will fails.
        assert!(main_pack.check().is_err());
        assert_eq!(
            main_pack.get_directory_pack_info(),
            &PackInfo {
                uuid: Uuid::from_bytes([
                    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
                    0x1d, 0x1e, 0x1f
                ]),
                pack_id: PackId::from(0),
                free_data: FreeData103::clone_from_slice(&[0xf0; 103]),
                pack_size: Size::new(0xffff),
                check_info_pos: Offset::new(0xff),
                pack_pos: PackPos::Offset(Offset::new(0xff00))
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
                free_data: FreeData103::clone_from_slice(&[0xf1; 103]),
                pack_size: Size::new(0xffffff),
                check_info_pos: Offset::new(0xff00ff),
                pack_pos: PackPos::Offset(Offset::new(0xff00))
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
                free_data: FreeData103::clone_from_slice(&[0xf2; 103]),
                pack_size: Size::new(0xffffff),
                check_info_pos: Offset::new(0xffffff),
                pack_pos: PackPos::Path("packpath".into())
            }
        );
        assert!(main_pack.get_content_pack_info(PackId::from(3)).is_err());
    }
}
*/
