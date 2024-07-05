use crate::reader::Stream;

use super::reader::*;
use super::types::*;
use super::Parsable;
use super::Parser;
use super::SizedParsable;
use super::SliceParser;
use super::{MemoryReader, Region, Source};
use std::sync::Arc;

// A wrapper around a source. Allowing access only on a region of the source
#[derive(Debug, Copy, Clone)]
pub struct SubReader<'s> {
    source: &'s Arc<dyn Source>,
    region: Region,
}

impl<'s> SubReader<'s> {
    pub fn new_from_arc(source: &'s Arc<dyn Source>, size: Size) -> Self {
        let region = Region::new_from_size(Offset::zero(), size);
        Self { source, region }
    }

    pub fn new_from_parts(source: &'s Arc<dyn Source>, region: Region) -> Self {
        Self { source, region }
    }

    pub fn to_owned(self) -> Reader {
        Reader::new_from_parts(Arc::clone(self.source), self.region)
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
        let region = self.region.cut_rel(offset, size);
        let slice = self.source.get_slice(region)?;
        Ok(SliceParser::new(slice, self.region.begin() + offset))
    }

    pub fn create_stream(&self, offset: Offset, size: Size) -> Stream {
        let region = self.region.cut_rel(offset, size);
        Stream::new_from_parts(Arc::clone(self.source), region, region.begin())
    }

    pub fn create_sub_reader(&self, offset: Offset, size: Size) -> SubReader<'s> {
        let region = self.region.cut_rel(offset, size);
        SubReader {
            source: self.source,
            region,
        }
    }
    pub fn create_sub_memory_reader(&self, offset: Offset, size: Size) -> Result<Reader> {
        let region = self.region.cut_rel(offset, size);
        let (source, region) = Arc::clone(self.source).into_memory_source(region)?;
        Ok(Reader::new_from_parts(source.into_source(), region))
    }

    pub fn into_memory_reader(self, offset: Offset, size: Size) -> Result<MemoryReader> {
        let region = self.region.cut_rel(offset, size);
        let (source, region) = Arc::clone(self.source).into_memory_source(region)?;
        Ok(MemoryReader::new_from_parts(source, region))
    }
}
