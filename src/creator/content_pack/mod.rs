mod cluster;
mod clusterwriter;
mod creator;

use crate::creator::InputReader;
use crate::{bases::*, ContentAddress};
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

pub enum CompHint {
    Yes,
    No,
    Detect,
}

impl Default for CompHint {
    fn default() -> Self {
        Self::Detect
    }
}

/// A trait for structure able to add content to a content pack.
///
/// Usefull to implement wrapper on one [ContentPackCreator]
pub trait ContentAdder {
    /// Add a content into a content pack.
    fn add_content<R: InputReader>(
        &mut self,
        reader: R,
        comp_hint: CompHint,
    ) -> Result<ContentAddress>;
}

pub struct CachedContentAdder<Wrapped: ContentAdder + 'static> {
    content_pack: Wrapped,
    cache: HashMap<blake3::Hash, ContentAddress>,
    progress: Rc<dyn CacheProgress>,
}

impl<Wrapped: ContentAdder> CachedContentAdder<Wrapped> {
    pub fn new(content_pack: Wrapped, progress: Rc<dyn CacheProgress>) -> Self {
        Self {
            content_pack,
            cache: Default::default(),
            progress,
        }
    }

    pub fn into_inner(self) -> Wrapped {
        self.content_pack
    }

    pub fn add_content<R: InputReader>(
        &mut self,
        mut reader: R,
        comp_hint: CompHint,
    ) -> Result<crate::ContentAddress> {
        let mut hasher = blake3::Hasher::new();
        hasher.update_reader(&mut reader)?;
        let hash = hasher.finalize();
        reader.seek(SeekFrom::Start(0))?;
        match self.cache.entry(hash) {
            Entry::Vacant(e) => {
                let content_address = self.content_pack.add_content(reader, comp_hint)?;
                e.insert(content_address);
                Ok(content_address)
            }
            Entry::Occupied(e) => {
                self.progress.cached_data(reader.size());
                Ok(*e.get())
            }
        }
    }
}

impl<Wrapper: ContentAdder> ContentAdder for CachedContentAdder<Wrapper> {
    fn add_content<R: InputReader>(
        &mut self,
        reader: R,
        comp_hint: CompHint,
    ) -> Result<crate::ContentAddress> {
        self.add_content(reader, comp_hint)
    }
}
