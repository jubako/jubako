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
        let o = offset.force_into_usize();
        let mut slice = &self.as_ref()[o..];
        match Read::read(&mut slice, buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }

    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let o = offset.force_into_usize();
        let e = o + buf.len();
        let our_size = self.as_ref().len();
        if e > our_size {
            return Err(format!("Out of slice. {e} ({o}) > {our_size}").into());
        }
        buf.copy_from_slice(&self.as_ref()[o..e]);
        Ok(())
    }

    fn get_slice(&self, region: ARegion, block_check: BlockCheck) -> Result<Cow<[u8]>> {
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
