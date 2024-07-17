use crate::reader::ByteSlice;
use crate::reader::Stream;

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
    pub fn new<T: Source + 'static>(source: T, size: Size) -> Self {
        Self::new_from_arc(Arc::new(source), size)
    }

    pub fn new_from_parts(source: Arc<dyn Source>, region: Region) -> Self {
        Self { source, region }
    }

    pub fn new_from_arc(source: Arc<dyn Source>, size: Size) -> Self {
        let region = Region::new_from_size(Offset::zero(), size);
        Self { source, region }
    }

    pub fn size(&self) -> Size {
        self.region.size()
    }

    pub fn parse_block_at<T: SizedBlockParsable>(&self, offset: Offset) -> Result<T::Output> {
        self.parse_block_in::<T>(offset, Size::from(T::SIZE))
    }

    pub fn parse_block_in<T: BlockParsable>(
        &self,
        offset: Offset,
        size: Size,
    ) -> Result<T::Output> {
        let check_reader = self.cut_check(offset, size)?;
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

    pub fn get_byte_slice(&self, offset: Offset, size: Size) -> ByteSlice {
        let region = self.region.cut_rel(offset, size);
        ByteSlice::new_from_parts(&self.source, region)
    }

    pub fn create_stream(&self, offset: Offset, size: Size) -> Stream {
        let region = self.region.cut_rel(offset, size);
        Stream::new_from_parts(Arc::clone(&self.source), region, region.begin())
    }

    pub fn cut(&self, offset: Offset, size: Size) -> Reader {
        let region = self.region.cut_rel(offset, size);
        Self::new_from_parts(Arc::clone(&self.source), region)
    }

    pub fn cut_check(&self, offset: Offset, size: Size) -> Result<CheckReader> {
        let region = self.region.cut_rel(offset, size);
        let (source, region) =
            Arc::clone(&self.source).into_memory_source(region, BlockCheck::Crc32)?;
        Ok(CheckReader::new_from_parts(source.into_source(), region))
    }

    pub fn create_sub_memory_reader(&self, offset: Offset, size: Size) -> Result<Reader> {
        let region = self.region.cut_rel(offset, size);
        let (source, region) =
            Arc::clone(&self.source).into_memory_source(region, BlockCheck::None)?;
        Ok(Reader {
            source: source.into_source(),
            region,
        })
    }
}

impl From<CheckReader> for Reader {
    fn from(breader: CheckReader) -> Self {
        Self::new_from_parts(breader.source, breader.region)
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
pub struct CheckReader {
    source: Arc<dyn Source>,
    region: Region,
}

impl CheckReader {
    pub fn new_from_parts(source: Arc<dyn Source>, region: Region) -> Self {
        Self { source, region }
    }

    pub(crate) fn create_parser(&self, offset: Offset, size: Size) -> Result<impl Parser + '_> {
        let region = self.region.cut_rel(offset, size);
        let slice = self.source.get_slice(region, BlockCheck::None)?;
        Ok(SliceParser::new(slice, self.region.begin() + offset))
    }

    pub fn size(&self) -> Size {
        self.region.size()
    }

    pub fn parse_at<T: super::SizedParsable>(&self, offset: Offset) -> Result<T::Output> {
        self.parse_in::<T>(offset, Size::from(T::SIZE))
    }

    pub fn parse_in<T: Parsable>(&self, offset: Offset, size: Size) -> Result<T::Output> {
        let mut parser = self.create_parser(offset, size)?;
        T::parse(&mut parser)
    }
    pub fn get_slice(&self, offset: Offset, size: Size) -> Result<Cow<[u8]>> {
        let region = self.region.cut_rel(offset, size);
        self.source.get_slice(region, BlockCheck::None)
    }
    pub fn get_byte_slice(&self, offset: Offset, size: Size) -> ByteSlice {
        let region = self.region.cut_rel(offset, size);
        ByteSlice::new_from_parts(&self.source, region)
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
