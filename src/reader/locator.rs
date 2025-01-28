use crate::bases::*;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub trait PackLocatorTrait: Sync + Send {
    fn locate(&self, uuid: Uuid, helper: &str) -> Result<Option<Reader>>;
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
    fn locate(&self, _uuid: Uuid, path: &str) -> Result<Option<Reader>> {
        let path = self.base_dir.join(path);
        if path.is_file() {
            Ok(Some(Reader::from(FileSource::open(path)?)))
        } else {
            Ok(None)
        }
    }
}

pub struct ChainedLocator(Vec<Arc<dyn PackLocatorTrait>>);

impl ChainedLocator {
    pub fn new(locators: Vec<Arc<dyn PackLocatorTrait>>) -> Self {
        Self(locators)
    }
}

impl PackLocatorTrait for ChainedLocator {
    fn locate(&self, uuid: Uuid, path: &str) -> Result<Option<Reader>> {
        for locator in &self.0 {
            let reader = locator.locate(uuid, path)?;
            if reader.is_some() {
                return Ok(reader);
            }
        }
        Ok(None)
    }
}
