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

    fn read(self: Arc<Self>, region: Region) -> Result<Box<dyn ReadSized>> {
        debug_assert!(region.end().force_into_usize() <= self.as_ref().as_ref().len());
        Ok(Box::new(ReadBuffer::new(self, region)))
    }

    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> std::io::Result<()> {
        let o = offset.force_into_usize();
        let e = o + buf.len();
        let our_size = self.as_ref().len();
        if e > our_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Out of slice. {e} ({o}) > {our_size}"),
            ));
        }
        buf.copy_from_slice(&self.as_ref()[o..e]);
        Ok(())
    }

    fn get_slice(&self, region: ARegion, block_check: BlockCheck) -> Result<Cow<'_, [u8]>> {
        debug_assert!(region.end().force_into_usize() <= self.as_ref().len());
        if let BlockCheck::Crc32 = block_check {
            let full_slice = &self.as_ref()[region.begin().force_into_usize()
                ..region.end().force_into_usize() + BlockCheck::Crc32.size()];
            assert_slice_crc(full_slice)?;
        }
        let slice =
            &self.as_ref()[region.begin().force_into_usize()..region.end().force_into_usize()];
        Ok(Cow::Borrowed(slice))
    }

    fn cut(
        self: Arc<Self>,
        region: Region,
        block_check: BlockCheck,
        _in_memory: bool,
    ) -> Result<(Arc<dyn Source>, Region)> {
        // THis will check the slice for us
        self.get_slice(region.try_into().unwrap(), block_check)?;
        Ok((self, region))
    }

    fn display(&self) -> String {
        format!("{:?}", self)
    }
}

struct ReadBuffer<T>
where
    T: AsRef<[u8]> + 'static + Sync + Send + std::fmt::Debug,
{
    source: Arc<T>,
    offset: Offset,
    region: Region,
}

impl<T> ReadBuffer<T>
where
    T: AsRef<[u8]> + 'static + Sync + Send + std::fmt::Debug,
{
    fn new(source: Arc<T>, region: Region) -> Self {
        Self {
            source,
            offset: region.begin(),
            region,
        }
    }
}

impl<T> ReadSized for ReadBuffer<T>
where
    T: AsRef<[u8]> + 'static + Sync + Send + std::fmt::Debug,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut full_slice = &self.source.as_ref().as_ref()
            [self.offset.force_into_usize()..self.region.end().force_into_usize()];
        let len = Read::read(&mut full_slice, buf)?;
        self.offset += len;
        Ok(len)
    }

    fn size_left(&self) -> Size {
        self.region.end() - self.offset
    }

    fn size(&self) -> Size {
        self.region.size()
    }

    fn offset(&self) -> Offset {
        (self.offset - self.region.begin()).into_u64().into()
    }
}
