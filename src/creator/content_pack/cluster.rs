use crate::bases::*;
use crate::common::ContentInfo;
use crate::creator::InputReader;

pub(super) struct ClusterCreator {
    compressed: bool,
    pub index: ClusterIdx,
    pub data: Vec<Box<dyn InputReader>>,
    pub offsets: Vec<usize>,
}

pub(crate) const CLUSTER_SIZE: Size = Size::new(1024 * 1024 * 4);
const MAX_BLOBS_PER_CLUSTER: usize = 0xFFF;

impl ClusterCreator {
    pub fn new(index: ClusterIdx, compressed: bool) -> Self {
        ClusterCreator {
            compressed,
            index,
            data: Vec::with_capacity(MAX_BLOBS_PER_CLUSTER),
            offsets: vec![],
        }
    }

    pub fn data_size(&self) -> Size {
        Size::from(*self.offsets.last().unwrap_or(&0))
    }

    pub fn is_full(&self, size: Size) -> bool {
        if self.offsets.len() == MAX_BLOBS_PER_CLUSTER {
            return true;
        }
        self.compressed && !self.offsets.is_empty() && self.data_size() + size > CLUSTER_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn add_content(&mut self, content: Box<dyn InputReader>) -> Result<ContentInfo> {
        assert!(self.offsets.len() < MAX_BLOBS_PER_CLUSTER);
        let content_size = content.size();
        let idx = self.offsets.len() as u16;
        let new_offset = self.offsets.last().unwrap_or(&0) + content_size.into_usize();
        self.data.push(content);
        self.offsets.push(new_offset);
        Ok(ContentInfo::new(self.index, BlobIdx::from(idx)))
    }
}
