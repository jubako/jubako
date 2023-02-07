use crate::bases::*;
use std::io::Read;
use std::sync::Arc;

impl<T: AsRef<[u8]> + 'static + Sync + Send> Source for T {
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

    fn into_memory(
        self: Arc<Self>,
        offset: Offset,
        size: usize,
    ) -> Result<(Arc<dyn Source>, Offset, End)> {
        debug_assert!(offset.into_usize() + size <= self.as_ref().as_ref().len());
        Ok((
            Arc::clone(&(self as Arc<dyn Source>)),
            offset,
            End::new_size(size as u64),
        ))
    }

    fn get_slice(&self, offset: Offset, end: Offset) -> Result<&[u8]> {
        debug_assert!(offset <= end);
        debug_assert!(end.into_usize() <= self.as_ref().len());
        Ok(&self.as_ref()[offset.into_usize()..end.into_usize()])
    }
}
