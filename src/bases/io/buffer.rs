use crate::bases::*;
use std::io::Read;
use std::rc::Rc;

impl<T: AsRef<[u8]> + 'static> Source for T {
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
        self: Rc<Self>,
        offset: Offset,
        size: usize,
    ) -> Result<(Rc<dyn Source>, Offset, End)> {
        assert!(offset.into_usize() + size <= self.as_ref().as_ref().len());
        Ok((
            Rc::clone(&(self as Rc<dyn Source>)),
            offset,
            End::new_size(size as u64),
        ))
    }

    fn get_slice(&self, offset: Offset, end: Offset) -> Result<&[u8]> {
        assert!(offset <= end);
        assert!(end.into_usize() <= self.as_ref().len());
        Ok(&self.as_ref()[offset.into_usize()..end.into_usize()])
    }
}
