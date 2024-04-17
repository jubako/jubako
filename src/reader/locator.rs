use crate::bases::*;
use bstr::ByteSlice;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

pub trait PackLocatorTrait: Sync + Send {
    fn locate(&self, uuid: Uuid, helper: &[u8]) -> Result<Option<Reader>>;
}

/** Locate in a directory
*/
pub struct FsLocator {
    base_dir: PathBuf,
}

impl FsLocator {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

impl PackLocatorTrait for FsLocator {
    fn locate(&self, _uuid: Uuid, path: &[u8]) -> Result<Option<Reader>> {
        let path = Path::new(path.to_path()?);
        let path = self.base_dir.join(path);
        if path.is_file() {
            Ok(Some(Reader::from(FileSource::open(path)?)))
        } else {
            Ok(None)
        }
    }
}

pub struct ChainedLocator(pub Vec<Arc<dyn PackLocatorTrait>>);

impl PackLocatorTrait for ChainedLocator {
    fn locate(&self, uuid: Uuid, path: &[u8]) -> Result<Option<Reader>> {
        for locator in &self.0 {
            let reader = locator.locate(uuid, path)?;
            if reader.is_some() {
                return Ok(reader);
            }
        }
        Ok(None)
    }
}
