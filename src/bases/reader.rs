use crate::reader::ByteSlice;

use super::flux::*;
use super::sub_reader::*;
use super::types::*;
use super::BlockParsable;
use super::DataBlockParsable;
use super::Parsable;
use super::SizedBlockParsable;
use super::SizedParsable;
use super::SliceParser;
use super::{MemoryReader, Region, Source};
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
        let region = self.region.cut_rel(offset, size);
        let slice = self.source.get_slice(region)?;
        let mut parser = SliceParser::new(slice, self.region.begin() + offset);
        T::parse(&mut parser)
    }

    pub(crate) fn parse_data_block<T: DataBlockParsable>(
        &self,
        sized_offset: SizedOffset,
    ) -> Result<T::Output> {
        let (intermediate, data_size) =
            self.parse_in::<T::TailParser>(sized_offset.offset, sized_offset.size)?;
        let data_reader = self.create_sub_reader(sized_offset.offset - data_size, data_size);
        T::finalize(intermediate, data_reader)
    }

    pub fn parse_at<T: SizedParsable>(&self, offset: Offset) -> Result<T::Output> {
        self.parse_in::<T>(offset, Size::from(T::SIZE))
    }

    pub fn parse_in<T: Parsable>(&self, offset: Offset, size: Size) -> Result<T::Output> {
        let mut flux = self.create_flux(offset, size);
        T::parse(&mut flux)
    }

    pub fn create_flux(&self, offset: Offset, size: Size) -> Flux {
        let region = self.region.cut_rel(offset, size);
        Flux::new_from_parts(&self.source, region, region.begin())
    }
    pub fn create_flux_for(&self, size_offset: SizedOffset) -> Flux {
        self.create_flux(size_offset.offset, size_offset.size)
    }
    pub fn create_flux_from(&self, offset: Offset) -> Flux {
        self.create_flux(offset, self.region.size() - offset.into())
    }
    pub fn create_flux_to(&self, size: Size) -> Flux {
        self.create_flux(Offset::zero(), size)
    }
    pub fn create_flux_all(&self) -> Flux {
        self.create_flux(Offset::zero(), self.region.size())
    }

    pub fn get_byte_slice(&self, offset: Offset, size: Size) -> ByteSlice {
        let region = self.region.cut_rel(offset, size);
        ByteSlice::new_from_parts(&self.source, region)
    }

    pub fn as_sub_reader(&self) -> SubReader {
        self.create_sub_reader(Offset::zero(), self.region.size())
    }

    pub fn create_sub_reader(&self, offset: Offset, size: Size) -> SubReader {
        let region = self.region.cut_rel(offset, size);
        SubReader::new_from_parts(&self.source, region)
    }

    pub fn create_sub_memory_reader(&self, offset: Offset, size: Size) -> Result<Reader> {
        let region = self.region.cut_rel(offset, size);
        let (source, region) = Arc::clone(&self.source).into_memory_source(region)?;
        Ok(Reader {
            source: source.into_source(),
            region,
        })
    }

    pub fn into_memory_reader(&self, offset: Offset, size: Size) -> Result<MemoryReader> {
        let region = self.region.cut_rel(offset, size);
        let (source, region) = Arc::clone(&self.source).into_memory_source(region)?;
        Ok(MemoryReader::new_from_parts(source, region))
    }
}

impl From<SubReader<'_>> for Reader {
    fn from(sub: SubReader) -> Self {
        sub.to_owned()
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
