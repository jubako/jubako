use crate::bases::*;
use std::{borrow::Cow, sync::Arc};

use super::{ByteSlice, Stream};

/// A conceptual slice of u8.
///
/// Constrarly to `&[u8]`, ByteRegion do not actually reference an existing slice.
/// It may reference:
/// - A mmap region of file
/// - A region of a file (to be read)
/// - The outcome of a (potentiall running) decompression process.
///
/// As [ByteSlice], ByteRegion is `Send` + `Sync`.
/// However, a ByteRegion has a `'static` lifetime. To do so, it clone a `Arc`, which may be a counter performance.
/// If you don't need to own the bytes, use a [ByteSlice].
#[derive(Debug, Clone)]
pub struct ByteRegion {
    pub(crate) source: Arc<dyn Source>,
    pub(crate) region: Region,
}

impl ByteRegion {
    /// The size of the region
    pub fn size(&self) -> Size {
        self.region.size()
    }

    /// Create a [Stream] (equivalent of `std::io::Cursor`)  for this ByteRegion
    pub fn stream(&self) -> Stream {
        Stream::new_from_parts(Arc::clone(&self.source), self.region, self.region.begin())
    }

    /// Create a new ByteRegion which is a subset of the current region.
    pub fn cut(&self, offset: Offset, size: Size) -> ByteSlice {
        let region = self.region.cut_rel(offset, size);
        ByteSlice {
            source: &self.source,
            region,
        }
    }

    pub fn as_slice(&self) -> ByteSlice {
        ByteSlice {
            source: &self.source,
            region: self.region,
        }
    }

    /// Get a slice to (a subset of) the bytes referenced by this region.
    ///
    /// Most of the time, it will return a `Cow::Borrowed` as ByteRegion actually reference data
    /// stored in memory but it may potentially be a `Cow::Owned` if it reference a file.
    pub fn get_slice(&self, offset: Offset, size: Size) -> Result<Cow<[u8]>> {
        let region = self.region.cut_rel(offset, size);
        self.source.get_slice(region)
    }
}

impl From<ByteSlice<'_>> for ByteRegion {
    fn from(byte_slice: ByteSlice<'_>) -> Self {
        Self {
            source: Arc::clone(byte_slice.source),
            region: byte_slice.region,
        }
    }
}

impl RandomParser for ByteRegion {
    type Parser<'s> = SliceParser<'s>;
    fn create_parser(&self, offset: Offset) -> Result<Self::Parser<'_>> {
        let region = self
            .region
            .cut_rel(offset, self.region.size() - offset.into());
        Ok(SliceParser::new(
            self.source.get_slice(region)?,
            self.region.begin() + offset,
        ))
    }

    fn read_slice(&self, offset: Offset, size: usize) -> Result<Cow<[u8]>> {
        let region = self.region.cut_rel(offset, Size::from(size));
        self.source.get_slice(region)
    }

    fn read_data(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        self.source.read_exact(self.region.begin() + offset, buf)
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for ByteRegion {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.get_slice(Offset::zero(), self.size()).unwrap())
    }
}

#[cfg(feature = "explorable")]
impl Explorable for ByteRegion {
    fn explore_one(&self, item: &str) -> Result<Option<Box<dyn Explorable>>> {
        Ok(Some(if item == "#" {
            Box::new(
                String::from_utf8_lossy(&self.get_slice(Offset::zero(), self.size())?).into_owned(),
            )
        } else {
            unreachable!()
        }))
    }
}