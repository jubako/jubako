use std::cell::OnceCell;

use super::content_pack::ContentPack;
use super::directory_pack::{DirectoryPack, EntryStorage};
use super::manifest_pack::ManifestPack;
use super::{Index, ValueStorage};
use crate::bases::*;
use crate::common::{ContentAddress, Pack, PackInfo, PackPos};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Container {
    path: PathBuf,
    main_pack: ManifestPack,
    reader: Reader,
    directory_pack: Arc<DirectoryPack>,
    value_storage: Arc<ValueStorage>,
    entry_storage: Arc<EntryStorage>,
    packs: Vec<OnceCell<ContentPack>>,
}

fn get_pack_reader(reader: &Reader, source_path: &Path, pack_info: &PackInfo) -> Result<Reader> {
    match &pack_info.pack_pos {
        PackPos::Offset(offset) => Ok(reader
            .create_sub_reader(*offset, End::Size(pack_info.pack_size))
            .into()),
        PackPos::Path(path) => {
            let path = source_path
                .parent()
                .unwrap()
                .join(OsString::from_vec(path.clone()));
            Ok(Reader::new(
                FileSource::open(path)?,
                End::Size(pack_info.pack_size),
            ))
        }
    }
}

pub fn locate_manifest(reader: Reader) -> Result<Reader> {
    let mut buffer_reader = [0u8; 64];

    // Check at beginning
    let mut flux = reader.create_flux_to(End::Size(Size::new(64)));
    flux.read_exact(&mut buffer_reader)?;
    if buffer_reader[0..4] == [0x6a, 0x62, 0x6b, 0x6d]
        || buffer_reader[0..4] == [0x6a, 0x62, 0x6b, 0x64]
        || buffer_reader[0..4] == [0x6a, 0x62, 0x6b, 0x63]
    {
        let size: Size = primitive::read_u64(&buffer_reader[32..40]).into();
        return Ok(reader
            .create_sub_reader(Offset::zero(), End::Size(size))
            .to_owned());
    };

    // Check at end
    let mut flux = reader.create_flux(
        (reader.size() - Size::new(64)).into(),
        End::Size(Size::new(64)),
    );
    flux.read_exact(&mut buffer_reader)?;
    buffer_reader.reverse();
    if buffer_reader[0..4] == [0x6a, 0x62, 0x6b, 0x6d]
        || buffer_reader[0..4] == [0x6a, 0x62, 0x6b, 0x64]
        || buffer_reader[0..4] == [0x6a, 0x62, 0x6b, 0x63]
    {
        let size: Size = primitive::read_u64(&buffer_reader[32..40]).into();
        let origin = reader.size() - size;
        return Ok(reader
            .create_sub_reader(origin.into(), End::Size(size))
            .into());
    };
    Err(Error::new(ErrorKind::NotAJbk))
}

impl Container {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path: PathBuf = path.as_ref().into();
        let reader = Reader::from(FileSource::open(&path)?);
        let reader = locate_manifest(reader)?;
        let main_pack =
            ManifestPack::new(reader.create_sub_memory_reader(Offset::zero(), End::None)?)?;
        let pack_info = main_pack.get_directory_pack_info();
        let directory_pack = Arc::new(DirectoryPack::new(get_pack_reader(
            &reader, &path, pack_info,
        )?)?);
        let value_storage = directory_pack.create_value_storage();
        let entry_storage = directory_pack.create_entry_storage();
        let mut packs = Vec::new();
        packs.resize_with((main_pack.max_id() + 1) as usize, Default::default);
        Ok(Self {
            path,
            main_pack,
            reader,
            directory_pack,
            value_storage,
            entry_storage,
            packs,
        })
    }
}

impl Container {
    pub fn pack_count(&self) -> PackCount {
        self.main_pack.pack_count()
    }

    pub fn get_pack(&self, pack_id: PackId) -> Result<&ContentPack> {
        self.packs[pack_id.into_usize()].get_or_try_init(|| self._get_pack(pack_id))
    }

    pub fn get_reader(&self, content: ContentAddress) -> Result<Reader> {
        let pack = self.get_pack(content.pack_id)?;
        pack.get_content(content.content_id)
    }

    fn _get_pack(&self, pack_id: PackId) -> Result<ContentPack> {
        let pack_info = self.main_pack.get_content_pack_info(pack_id)?;
        let pack_reader = self._get_pack_reader(pack_info)?;
        ContentPack::new(pack_reader)
    }

    pub fn get_directory_pack(&self) -> &Arc<DirectoryPack> {
        &self.directory_pack
    }

    pub fn get_value_storage(&self) -> &Arc<ValueStorage> {
        &self.value_storage
    }

    pub fn get_entry_storage(&self) -> &Arc<EntryStorage> {
        &self.entry_storage
    }

    pub fn get_index_for_name(&self, name: &str) -> Result<Index> {
        self.directory_pack.get_index_from_name(name)
    }

    fn _get_pack_reader(&self, pack_info: &PackInfo) -> Result<Reader> {
        get_pack_reader(&self.reader, &self.path, pack_info)
    }

    pub fn check(&self) -> Result<bool> {
        self.main_pack.check()
    }
}
