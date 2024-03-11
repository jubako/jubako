use super::{ContentPack, DirectoryPack, ManifestPack, PackLocatorTrait};
use crate::bases::*;
use crate::common::{ContainerPackHeader, FullPackKind, Pack, PackHeader, PackKind, PackLocator};
use serde::ser::SerializeMap;
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

impl serde::Serialize for ContainerPack {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut container = serializer.serialize_map(Some(self.packs.len()))?;
        for (uuid, reader) in self.packs.iter() {
            let pack_header =
                PackHeader::produce(&mut reader.create_flux_to(End::new_size(PackHeader::SIZE)))
                    .unwrap();
            container.serialize_entry(&uuid, &pack_header)?;
        }
        container.end()
    }
}

impl Explorable for ContainerPack {
    fn explore_one(&self, item: &str) -> Result<Option<Box<dyn Explorable>>> {
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
            return Err("Invalid key".into());
        };

        Ok(Some(
            match FullPackKind::produce(&mut reader.create_flux_all())? {
                PackKind::Manifest => Box::new(ManifestPack::new(reader.clone())?),
                PackKind::Directory => Box::new(DirectoryPack::new(reader.clone())?),
                PackKind::Content => Box::new(ContentPack::new(reader.clone())?),
                PackKind::Container => unreachable!(),
            },
        ))
    }
}
