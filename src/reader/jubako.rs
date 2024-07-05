use std::sync::OnceLock;

use super::container_pack::ContainerPack;
use super::content_pack::ContentPack;
use super::directory_pack::{DirectoryPack, EntryStorage};
use super::locator::{ChainedLocator, FsLocator, PackLocatorTrait};
use super::manifest_pack::ManifestPack;
use super::{ByteRegion, Index, MayMissPack, ValueStorage};
use crate::bases::*;
use crate::common::{ContentAddress, FullPackKind, Pack, PackKind};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use zerocopy::little_endian::U64;
use zerocopy::FromBytes;

/// Container is the main structure which reunit other structures to read a Jubako container.
///
/// While it is possible to use all other structures in [`reader`] module, `Container` provides
/// a uniform way to read a Jubako container.
///
/// It takes at its charge:
/// - Loading of different packs
/// - Storage of opened value and entry stores
/// - Access to [`Reader`] for a specific [`ContentAddress`]
pub struct Container {
    manifest_pack: ManifestPack,
    locator: Arc<dyn PackLocatorTrait>,
    directory_pack: Arc<DirectoryPack>,
    value_storage: Arc<ValueStorage>,
    entry_storage: Arc<EntryStorage>,
    packs: Vec<OnceLock<ContentPack>>,
}

fn parse_header(buffer: [u8; 64]) -> Result<(PackKind, Size)> {
    let reader: Reader = buffer.into();
    let kind = reader.parse_at::<FullPackKind>(Offset::zero())?;
    Ok(match kind {
        PackKind::Directory | PackKind::Content => (kind, Size::zero()),
        PackKind::Container => {
            let size = U64::read_from(&buffer[8..16]).unwrap();
            let size: Size = size.get().into();
            (kind, size)
        }
        PackKind::Manifest => {
            let size = U64::read_from(&buffer[32..40]).unwrap();
            let size: Size = size.get().into();
            (kind, size)
        }
    })
}

pub fn locate_pack(reader: Reader) -> Result<ContainerPack> {
    let mut buffer_reader = [0u8; 64];

    // Check at beginning
    {
        let mut stream = reader.create_stream(Offset::zero(), Size::new(64));
        stream.read_exact(&mut buffer_reader)?;
    }
    let (kind, offset, size) = match parse_header(buffer_reader) {
        Ok((kind, size)) => (kind, Offset::zero(), size),
        Err(_) => {
            //Check at end
            let mut stream =
                reader.create_stream((reader.size() - Size::new(64)).into(), Size::new(64));
            stream.read_exact(&mut buffer_reader)?;
            buffer_reader.reverse();
            let (kind, size) = parse_header(buffer_reader)?;
            let origin = reader.size() - size;
            (kind, origin.into(), size)
        }
    };

    match kind {
        PackKind::Directory | PackKind::Content => Err("Not a valid pack".into()),
        PackKind::Container => ContainerPack::new(reader.create_sub_reader(offset, size).into()),
        PackKind::Manifest => {
            let uuid = Uuid::from_bytes(buffer_reader[10..26].try_into().unwrap());
            Ok(ContainerPack::new_fake(reader, uuid))
        }
    }
}

impl Container {
    /// Open a new container
    ///
    /// `path` is the path to the manifest pack (or a container pack with a manifest pack within).
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let locator = Arc::new(FsLocator::new(
            path.as_ref().parent().unwrap().to_path_buf(),
        ));
        Self::new_with_locator(path, locator)
    }

    /// Open a new container with a specific locator to found other pack.
    ///
    /// `path` is the path to the manifest pack (or a container pack with a manifest pack within).
    pub fn new_with_locator<P: AsRef<Path>>(
        path: P,
        locator: Arc<dyn PackLocatorTrait>,
    ) -> Result<Self> {
        let path: PathBuf = path.as_ref().into();
        let reader = Reader::from(FileSource::open(path)?);
        let container_pack = Arc::new(locate_pack(reader)?);
        let reader = container_pack
            .get_pack_reader_from_idx(PackId::from(container_pack.pack_count().into_u16() - 1))
            .unwrap();

        let manifest_pack =
            ManifestPack::new(reader.create_sub_memory_reader(Offset::zero(), reader.size())?)?;

        let locators: Vec<Arc<dyn PackLocatorTrait>> = vec![container_pack, locator];
        let locator = Arc::new(ChainedLocator(locators));

        let pack_info = manifest_pack.get_directory_pack_info();
        let directory_pack = Arc::new(DirectoryPack::new(
            locator
                .locate(pack_info.uuid, &pack_info.pack_location)?
                .unwrap(),
        )?);
        let value_storage = directory_pack.create_value_storage();
        let entry_storage = directory_pack.create_entry_storage();
        let mut packs = Vec::new();
        packs.resize_with((manifest_pack.max_id() + 1) as usize, Default::default);
        Ok(Self {
            manifest_pack,
            locator,
            directory_pack,
            value_storage,
            entry_storage,
            packs,
        })
    }
}

impl Container {
    /// The number of packs in the container.
    pub fn pack_count(&self) -> PackCount {
        self.manifest_pack.pack_count()
    }

    pub fn get_pack(&self, pack_id: PackId) -> Result<MayMissPack<&ContentPack>> {
        let cache_slot = &self.packs[pack_id.into_usize()];
        if cache_slot.get().is_none() {
            match self._get_pack(pack_id)? {
                MayMissPack::MISSING(pack_info) => return Ok(MayMissPack::MISSING(pack_info)),
                MayMissPack::FOUND(p) => {
                    let _ = cache_slot.set(p);
                }
            }
        }
        Ok(MayMissPack::FOUND(cache_slot.get().unwrap()))
    }

    pub fn get_bytes(&self, content: ContentAddress) -> Result<MayMissPack<ByteRegion>> {
        let pack = self.get_pack(content.pack_id)?;
        pack.map(|p| p.get_content(content.content_id)).transpose()
    }

    fn _get_pack(&self, pack_id: PackId) -> Result<MayMissPack<ContentPack>> {
        let pack_info = self.manifest_pack.get_content_pack_info(pack_id)?;
        let pack_reader = self
            .locator
            .locate(pack_info.uuid, &pack_info.pack_location)?;
        match pack_reader {
            None => Ok(MayMissPack::MISSING(pack_info.clone())),
            Some(r) => Ok(MayMissPack::FOUND(ContentPack::new(r)).transpose()?),
        }
    }

    /// Get the directory pack of the container
    pub fn get_directory_pack(&self) -> &Arc<DirectoryPack> {
        &self.directory_pack
    }

    /// Get the value storage of the container
    pub fn get_value_storage(&self) -> &Arc<ValueStorage> {
        &self.value_storage
    }

    /// Get the entry storage of the container
    pub fn get_entry_storage(&self) -> &Arc<EntryStorage> {
        &self.entry_storage
    }

    /// Get a index by its name
    pub fn get_index_for_name(&self, name: &str) -> Result<Index> {
        self.directory_pack.get_index_from_name(name)
    }

    /// Check the container
    pub fn check(&self) -> Result<bool> {
        self.manifest_pack.check()
    }

    /// Get the uuid of the container (manifest_pack)
    pub fn uuid(&self) -> Uuid {
        self.manifest_pack.uuid()
    }
}
