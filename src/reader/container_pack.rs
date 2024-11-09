use super::{ContentPack, DirectoryPack, ManifestPack, PackLocatorTrait};
use crate::bases::*;
use crate::common::{ContainerPackHeader, Pack, PackHeader, PackKind, PackLocator};
use std::collections::HashMap;
use uuid::Uuid;

pub struct ContainerPack {
    packs_uuid: Vec<Uuid>,
    packs: HashMap<Uuid, Reader>,
}

impl ContainerPack {
    pub fn new(reader: Reader) -> Result<Self> {
        let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero())?;
        if pack_header.magic != PackKind::Container {
            return Err(format_error!("Pack Magic is not Container Pack"));
        }

        let header =
            reader.parse_block_at::<ContainerPackHeader>(Offset::from(PackHeader::BLOCK_SIZE))?;
        let mut pack_offset = header.pack_locators_pos;
        let mut packs_uuid = Vec::with_capacity(header.pack_count.into_usize());
        let mut packs = HashMap::with_capacity(header.pack_count.into_usize());
        for _idx in header.pack_count {
            let pack_locator = reader.parse_block_at::<PackLocator>(pack_offset)?;
            pack_offset += PackLocator::BLOCK_SIZE;
            let pack_reader = reader.cut(pack_locator.pack_pos, pack_locator.pack_size, false)?;
            packs.insert(pack_locator.uuid, pack_reader);
            packs_uuid.push(pack_locator.uuid);
        }
        Ok(Self { packs_uuid, packs })
    }

    pub fn new_fake(reader: Reader, uuid: Uuid) -> Self {
        let mut packs_uuid = Vec::with_capacity(1);
        let mut packs = HashMap::with_capacity(1);
        packs.insert(uuid, reader);
        packs_uuid.push(uuid);
        Self { packs_uuid, packs }
    }

    pub fn pack_count(&self) -> PackCount {
        (self.packs.len() as u16).into()
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

    pub fn iter(&self) -> impl Iterator<Item = (&Uuid, &Reader)> {
        self.packs.iter()
    }

    pub fn get_manifest_pack_reader(&self) -> Result<Option<Reader>> {
        for reader in self.packs.values() {
            let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero())?;
            if let PackKind::Manifest = pack_header.magic {
                return Ok(Some(reader.clone()));
            }
        }
        Ok(None)
    }

    pub fn check(&self) -> Result<bool> {
        for reader in self.packs.values() {
            let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero())?;
            let ok = match pack_header.magic {
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

#[cfg(feature = "explorable_serde")]
impl serde::Serialize for ContainerPack {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut container = serializer.serialize_map(Some(self.packs.len()))?;
        for (uuid, reader) in self.packs.iter() {
            let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero()).unwrap();
            container.serialize_entry(&uuid, &pack_header)?;
        }
        container.end()
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for ContainerPack {
    fn header_footer(&self) -> Option<(String, String)> {
        Some(("Packs(".to_string(), ")".to_string()))
    }
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        use yansi::Paint;
        for (_uuid, reader) in self.packs.iter() {
            let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero()).unwrap();
            out.field(&pack_header.uuid.bold(), &pack_header)?;
        }
        Ok(())
    }
}

#[cfg(feature = "explorable")]
impl graphex::Node for ContainerPack {
    fn next(&self, item: &str) -> graphex::ExploreResult {
        let reader = if let Ok(index) = item.parse::<usize>() {
            let uuid = self
                .packs_uuid
                .get(index)
                .ok_or_else(|| Error::from(format!("{item} is not a valid key.")))?;
            &self.packs[uuid]
        } else if let Ok(uuid) = item.parse::<Uuid>() {
            self.packs
                .get(&uuid)
                .ok_or_else(|| Error::from(format!("{item} is not a valid key.")))?
        } else {
            return Err(graphex::Error::key("Invalid key"));
        };

        Ok(
            match reader.parse_block_at::<PackHeader>(Offset::zero())?.magic {
                PackKind::Manifest => Box::new(ManifestPack::new(reader.clone())?).into(),
                PackKind::Directory => Box::new(DirectoryPack::new(reader.clone())?).into(),
                PackKind::Content => Box::new(ContentPack::new(reader.clone())?).into(),
                PackKind::Container => unreachable!(),
            },
        )
    }

    fn display(&self) -> &dyn graphex::Display {
        self
    }

    #[cfg(feature = "explorable_serde")]
    fn serde(&self) -> Option<&dyn erased_serde::Serialize> {
        Some(self)
    }
}
