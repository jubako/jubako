use std::cell::OnceCell;

use super::content_pack::ContentPack;
use super::directory_pack::{Content, DirectoryPack};
use super::manifest_pack::{ManifestPack, PackInfo};
use crate::bases::*;
use crate::common::{Pack, PackPos};
use std::ffi::OsString;
use std::fs::File;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct Container {
    path: PathBuf,
    main_pack: ManifestPack,
    reader: FileReader,
    directory_pack: OnceCell<Rc<DirectoryPack>>,
    packs: Vec<OnceCell<ContentPack>>,
}

impl Container {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path: PathBuf = path.as_ref().into();
        let file = File::open(path.clone())?;
        let reader = FileReader::new(file, End::None);
        let main_pack =
            ManifestPack::new(reader.create_sub_memory_reader(Offset::from(0_u64), End::None)?)?;
        let mut packs = Vec::new();
        packs.resize_with((main_pack.max_id() + 1) as usize, Default::default);
        Ok(Self {
            path,
            main_pack,
            reader,
            directory_pack: OnceCell::new(),
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

    pub fn get_reader(&self, content: &Content) -> Result<Box<dyn Reader>> {
        let pack = self.get_pack(content.pack_id())?;
        pack.get_content(content.content_id())
    }

    fn _get_pack(&self, pack_id: PackId) -> Result<ContentPack> {
        let pack_info = self.main_pack.get_content_pack_info(pack_id)?;
        let pack_reader = self._get_pack_reader(pack_info)?;
        ContentPack::new(pack_reader)
    }

    pub fn get_directory_pack(&self) -> Result<&Rc<DirectoryPack>> {
        self.directory_pack
            .get_or_try_init(|| self._get_directory_pack())
    }

    fn _get_directory_pack(&self) -> Result<Rc<DirectoryPack>> {
        let pack_info = self.main_pack.get_directory_pack_info();
        let pack_reader = self._get_pack_reader(pack_info)?;
        Ok(Rc::new(DirectoryPack::new(pack_reader)?))
    }

    fn _get_pack_reader(&self, pack_info: &PackInfo) -> Result<Box<dyn Reader>> {
        match &pack_info.pack_pos {
            PackPos::Offset(offset) => Ok(self
                .reader
                .create_sub_reader(*offset, End::Size(pack_info.pack_size))),
            PackPos::Path(path) => {
                let path = self
                    .path
                    .parent()
                    .unwrap()
                    .join(OsString::from_vec(path.clone()));
                let file = File::open(path)?;
                Ok(Box::new(FileReader::new(
                    file,
                    End::Size(pack_info.pack_size),
                )))
            }
        }
    }

    pub fn check(&self) -> Result<bool> {
        self.main_pack.check()
    }
}
