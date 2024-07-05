use super::{types::*, Parsable, Parser, SizedParsable, SliceParser};
use super::{MemorySource, Reader, Region};
use std::borrow::Cow;
use std::sync::Arc;

// A wrapper around a source. Allowing access only on a region of the source
#[derive(Debug)]
pub struct MemoryReader {
    source: Arc<dyn MemorySource>,
    region: Region,
}

impl MemoryReader {
    pub fn new<T: MemorySource + 'static>(source: T, size: Size) -> Self {
        Self::new_from_arc(Arc::new(source), size)
    }

    pub fn new_from_parts(source: Arc<dyn MemorySource>, region: Region) -> Self {
        Self { source, region }
    }

    pub fn new_from_arc(source: Arc<dyn MemorySource>, size: Size) -> Self {
        let region = Region::new_from_size(Offset::zero(), size);
        Self { source, region }
    }

    pub fn size(&self) -> Size {
        self.region.size()
    }

    pub fn parse_at<T: SizedParsable>(&self, offset: Offset) -> Result<T::Output> {
        self.parse_in::<T>(offset, Size::from(T::SIZE))
    }

    pub fn parse_in<T: Parsable>(&self, offset: Offset, size: Size) -> Result<T::Output> {
        let mut parser = self.create_parser(offset, size)?;
        T::parse(&mut parser)
    }

    pub fn create_parser(&self, offset: Offset, size: Size) -> Result<impl Parser + '_> {
        Ok(SliceParser::new(
            Cow::Borrowed(self.get_slice(offset, size)?),
            self.region.begin() + offset,
        ))
    }

    /// Get a slice from the reader.
    /// This is usefull only if this is a memory reader, panic if not
    /// [TODO] Use a new trait/type for this.
    pub fn get_slice(&self, offset: Offset, size: Size) -> Result<&[u8]> {
        let region = self.region.cut_rel(offset, size);
        // We know for sure that our reader in inside the region of our source.
        // This is also true for SeekableDecoder as it has already called
        // decode_to before casting itself to a MemorySource
        unsafe { self.source.get_slice_unchecked(region) }
    }
}

impl TryFrom<Reader> for MemoryReader {
    type Error = Error;
    fn try_from(reader: Reader) -> Result<Self> {
        reader.into_memory_reader(Offset::zero(), reader.size())
    }
}
