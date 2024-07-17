use crate::bases::*;
use std::sync::Arc;
use std::{borrow::Cow, io::Read};

impl<T> Source for T
where
    T: AsRef<[u8]> + 'static + Sync + Send + std::fmt::Debug,
{
    fn size(&self) -> Size {
        self.as_ref().len().into()
    }
    fn read(&self, offset: Offset, buf: &mut [u8]) -> Result<usize> {
        let o = offset.into_usize();
        let mut slice = &self.as_ref()[o..];
        match Read::read(&mut slice, buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }

    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let o = offset.into_usize();
        let e = o + buf.len();
        let our_size = self.as_ref().len();
        if e > our_size {
            return Err(format!("Out of slice. {e} ({o}) > {our_size}").into());
        }
        buf.copy_from_slice(&self.as_ref()[o..e]);
        Ok(())
    }

    fn get_slice(&self, region: Region, block_check: BlockCheck) -> Result<Cow<[u8]>> {
        debug_assert!(region.end().into_usize() <= self.as_ref().len());
        if let BlockCheck::Crc32 = block_check {
            if self.as_ref()[region.end().into_usize()..region.end().into_usize() + 4] != [0; 4] {
                return Err(format_error!("Not a valid checksum"));
            }
        }
        Ok(Cow::Borrowed(
            &self.as_ref()[region.begin().into_usize()..region.end().into_usize()],
        ))
    }

    fn into_memory_source(
        self: Arc<Self>,
        region: Region,
        block_check: BlockCheck,
    ) -> Result<(Arc<dyn MemorySource>, Region)> {
        debug_assert!(region.end().into_usize() <= self.as_ref().as_ref().len());
        if let BlockCheck::Crc32 = block_check {
            if self.as_ref().as_ref()[region.end().into_usize()..region.end().into_usize() + 4]
                != [0; 4]
            {
                return Err(format_error!("Not a valid checksum"));
            }
        }
        Ok((Arc::clone(&(self as Arc<dyn MemorySource>)), region))
    }

    fn display(&self) -> String {
        format!("{:?}", self)
    }
}

impl<T> MemorySource for T
where
    T: AsRef<[u8]> + 'static + Sync + Send + std::fmt::Debug,
{
    fn get_slice(&self, region: Region) -> Result<&[u8]> {
        debug_assert!(region.end().into_usize() <= self.as_ref().len());
        Ok(&self.as_ref()[region.begin().into_usize()..region.end().into_usize()])
    }
    unsafe fn get_slice_unchecked(&self, region: Region) -> Result<&[u8]> {
        debug_assert!(region.end().into_usize() <= self.as_ref().len());
        Ok(&self.as_ref()[region.begin().into_usize()..region.end().into_usize()])
    }
    fn into_source(self: Arc<Self>) -> Arc<dyn Source> {
        self
    }
}
