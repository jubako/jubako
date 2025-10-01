use crate::reader::ByteSlice;
use crate::reader::ByteStream;

use super::types::*;
use super::BlockCheck;
use super::BlockParsable;
use super::DataBlockParsable;
use super::Parsable;
use super::Parser;
use super::SizedBlockParsable;
use super::SliceParser;
use super::{Region, Source};
use std::borrow::Cow;
use std::sync::Arc;

// A wrapper around a source. Allowing access only on a region of the source
#[derive(Debug, Clone)]
pub struct Reader {
    source: Arc<dyn Source>,
    region: Region,
}

impl Reader {
    pub(crate) fn new<T: Source + 'static>(source: T, size: Size) -> Self {
        Self::new_from_arc(Arc::new(source), size)
    }

    pub(crate) fn new_from_parts(source: Arc<dyn Source>, region: Region) -> Self {
        Self { source, region }
    }

    pub(crate) fn new_from_arc(source: Arc<dyn Source>, size: Size) -> Self {
        let region = Region::new_from_size(Offset::zero(), size);
        Self { source, region }
    }

    pub(crate) fn global_offset(&self) -> Offset {
        self.region.begin()
    }

    pub(crate) fn size(&self) -> Size {
        self.region.size()
    }

    pub(crate) fn parse_block_unchecked_at<T: SizedBlockParsable>(
        &self,
        offset: Offset,
    ) -> Result<T::Output> {
        let check_reader = self.cut_check(offset, T::SIZE.into(), BlockCheck::None)?;
        check_reader.parse_in::<T>(Offset::zero(), T::SIZE.into())
    }

    pub(crate) fn parse_block_at<T: SizedBlockParsable>(
        &self,
        offset: Offset,
    ) -> Result<T::Output> {
        self.parse_block_in::<T>(offset, T::SIZE.into())
    }

    pub(crate) fn parse_block_in<T: BlockParsable>(
        &self,
        offset: Offset,
        size: ASize,
    ) -> Result<T::Output> {
        let check_reader = self.cut_check(offset, size.into(), BlockCheck::Crc32)?;
        check_reader.parse_in::<T>(Offset::zero(), size)
    }

    pub(crate) fn parse_data_block<T: DataBlockParsable>(
        &self,
        sized_offset: SizedOffset,
    ) -> Result<T::Output> {
        let intermediate =
            self.parse_block_in::<T::TailParser>(sized_offset.offset, sized_offset.size)?;
        T::finalize(intermediate, sized_offset.offset, self)
    }

    pub(crate) fn get_byte_slice(&self, offset: Offset, size: Size) -> ByteSlice<'_> {
        let region = self.region.cut_rel(offset, size);
        ByteSlice::new_from_parts(&self.source, region)
    }

    pub(crate) fn create_stream(
        &self,
        offset: Offset,
        size: Size,
        in_memory: bool,
    ) -> Result<ByteStream> {
        let (source, region) = self.cut_source(offset, size, BlockCheck::None, in_memory)?;
        Ok(ByteStream::new_from_parts(source, region, region.begin()))
    }

    #[inline]
    fn cut_source(
        &self,
        offset: Offset,
        size: Size,
        block_check: BlockCheck,
        in_memory: bool,
    ) -> Result<(Arc<dyn Source>, Region)> {
        let region = self.region.cut_rel(offset, size);
        Arc::clone(&self.source).cut(region, block_check, in_memory)
    }

    #[inline]
    pub(crate) fn cut(&self, offset: Offset, size: Size, in_memory: bool) -> Result<Reader> {
        let (source, region) = self.cut_source(offset, size, BlockCheck::None, in_memory)?;
        Ok(Self::new_from_parts(source, region))
    }

    pub(crate) fn cut_check(
        &self,
        offset: Offset,
        size: Size,
        block_check: BlockCheck,
    ) -> Result<CheckReader> {
        let (source, region) = self.cut_source(offset, size, block_check, true)?;
        Ok(CheckReader::new_from_parts(source, region))
    }
}

impl From<CheckReader> for Reader {
    fn from(creader: CheckReader) -> Self {
        Self::new_from_parts(creader.source, creader.region)
    }
}

impl<T> From<T> for Reader
where
    T: Source + 'static,
{
    fn from(source: T) -> Self {
        let size = source.size();
        Self::new(source, size)
    }
}

impl std::fmt::Display for Reader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at range [{}, {}]",
            self.source.display(),
            self.region.begin().into_u64(),
            self.region.end().into_u64()
        )
    }
}

/// A wrapper around a source. Allowing access only on a region of the source
/// A `CheckReader` provides access to a checked region (ie: CRC32 is verified%)
#[derive(Debug, Clone)]
pub(crate) struct CheckReader {
    source: Arc<dyn Source>,
    region: Region,
}

impl CheckReader {
    pub fn new_from_parts(source: Arc<dyn Source>, region: Region) -> Self {
        Self { source, region }
    }

    pub(crate) fn create_parser(&self, offset: Offset, size: ASize) -> Result<impl Parser + '_> {
        let region = self.region.cut_rel_asize(offset, size);
        let slice = self.source.get_slice(region, BlockCheck::None)?;
        Ok(SliceParser::new(slice, self.region.begin() + offset))
    }

    #[cfg(any(feature = "explorable", test))]
    pub fn size(&self) -> Size {
        self.region.size()
    }

    pub fn parse_in<T: Parsable>(&self, offset: Offset, size: ASize) -> Result<T::Output> {
        let mut parser = self.create_parser(offset, size)?;
        T::parse(&mut parser)
    }
    pub fn get_slice(&self, offset: Offset, size: ASize) -> Result<Cow<'_, [u8]>> {
        let region = self.region.cut_rel_asize(offset, size);
        self.source.get_slice(region, BlockCheck::None)
    }
}

#[cfg(test)]
impl<T> From<T> for CheckReader
where
    T: Source + 'static,
{
    fn from(source: T) -> Self {
        let region = Region::new_from_size(Offset::zero(), source.size());
        Self {
            source: Arc::new(source),
            region,
        }
    }
}
