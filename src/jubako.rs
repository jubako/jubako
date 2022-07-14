use std::cell::OnceCell;

use crate::bases::*;
use crate::content_pack::ContentPack;
use crate::directory_pack::DirectoryPack;
use crate::main_pack::{MainPack, PackInfo, PackPos};
use crate::pack::Pack;
use std::fs::File;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

pub struct Container {
    path: PathBuf,
    main_pack: MainPack,
    reader: FileReader,
    directory_pack: OnceCell<DirectoryPack>,
    packs: Vec<OnceCell<ContentPack>>,
}

impl Container {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path: PathBuf = path.as_ref().into();
        let file = File::open(path.clone())?;
        let reader = FileReader::new(file, End::None);
        let main_pack = MainPack::new(reader.create_sub_reader(Offset(0), End::None))?;
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
    pub fn pack_count(&self) -> u8 {
        self.main_pack.pack_count()
    }

    pub fn get_pack(&self, pack_id: u8) -> Result<&ContentPack> {
        self.packs[pack_id as usize].get_or_try_init(|| self._get_pack(pack_id))
    }

    fn _get_pack(&self, pack_id: u8) -> Result<ContentPack> {
        let pack_info = self.main_pack.get_content_pack_info(pack_id)?;
        let pack_reader = self._get_pack_reader(pack_info)?;
        ContentPack::new(pack_reader)
    }

    pub fn get_directory_pack(&self) -> Result<&DirectoryPack> {
        self.directory_pack
            .get_or_try_init(|| self._get_directory_pack())
    }

    fn _get_directory_pack(&self) -> Result<DirectoryPack> {
        let pack_info = self.main_pack.get_directory_pack_info();
        let pack_reader = self._get_pack_reader(pack_info)?;
        DirectoryPack::new(pack_reader)
    }

    fn _get_pack_reader(&self, pack_info: &PackInfo) -> Result<Box<dyn Reader>> {
        match &pack_info.pack_pos {
            PackPos::Offset(offset) => Ok(self
                .reader
                .create_sub_reader(*offset, End::Size(pack_info.pack_size))),
            PackPos::Path(path) => {
                let path = self.path.parent().unwrap().join(OsString::from_vec(path.clone()));
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
