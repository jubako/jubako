mod cluster;
mod clusterwriter;
mod creator;

use crate::bases::*;
use crate::creator::InputReader;
pub use creator::ContentPackCreator;
use std::collections::{hash_map::Entry, HashMap};
use std::io::SeekFrom;
use std::rc::Rc;

pub trait Progress: Send + Sync {
    fn new_cluster(&self, _cluster_idx: u32, _compressed: bool) {}
    fn handle_cluster(&self, _cluster_idx: u32, _compressed: bool) {}
    fn handle_cluster_written(&self, _cluster_idx: u32) {}
    fn content_added(&self, _size: Size) {}
}

impl Progress for () {}

pub trait CacheProgress {
    fn cached_data(&self, _size: Size) {}
}

impl CacheProgress for () {}

pub struct CachedContentPackCreator<O: OutStream + 'static> {
    content_pack: ContentPackCreator<O>,
    cache: HashMap<blake3::Hash, ContentIdx>,
    progress: Rc<dyn CacheProgress>,
}

impl<O: OutStream> CachedContentPackCreator<O> {
    pub fn new(content_pack: ContentPackCreator<O>, progress: Rc<dyn CacheProgress>) -> Self {
        Self {
            content_pack,
            cache: Default::default(),
            progress,
        }
    }

    pub fn add_content<R>(&mut self, mut content: R) -> Result<ContentIdx>
    where
        R: InputReader + 'static,
    {
        let mut hasher = blake3::Hasher::new();
        hasher.update_reader(&mut content)?;
        let hash = hasher.finalize();
        content.seek(SeekFrom::Start(0))?;
        match self.cache.entry(hash) {
            Entry::Vacant(e) => {
                let content_idx = self.content_pack.add_content(content)?;
                e.insert(content_idx);
                Ok(content_idx)
            }
            Entry::Occupied(e) => {
                self.progress.cached_data(content.size());
                Ok(*e.get())
            }
        }
    }

    pub fn into_inner(self) -> ContentPackCreator<O> {
        self.content_pack
    }
}
