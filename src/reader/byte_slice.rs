use crate::bases::*;
use std::{borrow::Cow, sync::Arc};

use super::ByteStream;

/// A conceptual slice of u8.
///
/// Constrarly to `&[u8]`, ByteSlice do not actually reference an existing slice.
/// It may reference:
/// - A mmap region of file
/// - A region of a file (to be read)
/// - The outcome of a (potentiall running) decompression process.
///
/// On top of that, ByteSlice is `'static` and it is `Send` + `Sync`.
#[derive(Debug, Clone)]
pub struct ByteSlice<'s> {
    pub(crate) source: &'s Arc<dyn Source>,
    pub(crate) region: Region,
}

impl<'s> ByteSlice<'s> {
    pub(crate) fn new_from_parts(source: &'s Arc<dyn Source>, region: Region) -> Self {
        Self { source, region }
    }

    /// The size of the slice
    pub fn size(&self) -> Size {
        self.region.size()
    }

    /// Create a [ByteStream] (equivalent of `std::io::Cursor`)  for this ByteSlice
    pub fn stream(&self) -> ByteStream {
        ByteStream::new_from_parts(Arc::clone(self.source), self.region, self.region.begin())
    }

    /// Create a new ByteSlice which is a subset of the current region.
    pub fn cut(&self, offset: Offset, size: Size) -> Self {
        let region = self.region.cut_rel(offset, size);
        Self {
            source: self.source,
            region,
        }
    }

    /// Get a slice to (a subset of) the bytes referenced by this region.
    ///
    /// Most of the time, it will return a `Cow::Borrowed` as ByteSlice actually reference data
    /// stored in memory but it may potentially be a `Cow::Owned` if it reference a file.
    pub fn get_slice(&self, offset: Offset, size: usize) -> Result<Cow<'_, [u8]>> {
        let region = self.region.cut_rel_asize(offset, ASize::new(size));
        self.source.get_slice(region, BlockCheck::None)
    }
}

impl<'s> RandomParser for ByteSlice<'s> {
    type Parser<'p>
        = SliceParser<'p>
    where
        's: 'p;
    fn create_parser(&self, offset: Offset) -> Result<Self::Parser<'_>> {
        let size = self.region.size() - offset.into();
        let size = std::cmp::min(0xFFFF_u64, size.into_u64()) as usize;
        let region = self.region.cut_rel_asize(offset, size.into());
        Ok(SliceParser::new(
            self.source.get_slice(region, BlockCheck::None)?,
            self.region.begin() + offset,
        ))
    }

    fn global_offset(&self) -> Offset {
        self.region.begin()
    }

    fn read_slice(&self, offset: Offset, size: usize) -> Result<Cow<'_, [u8]>> {
        let region = self.region.cut_rel_asize(offset, ASize::from(size));
        self.source.get_slice(region, BlockCheck::None)
    }

    fn read_data(&self, offset: Offset, buf: &mut [u8]) -> IoResult<()> {
        self.source.read_exact(self.region.begin() + offset, buf)
    }
}
