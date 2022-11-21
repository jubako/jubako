use crate::bases::*;
use memmap2::Mmap;
use std::io::Read;
use std::rc::Rc;

impl Source for Mmap {
    fn size(&self) -> Size {
        self.len().into()
    }
    fn read(&self, offset: Offset, buf: &mut [u8]) -> Result<usize> {
        let o = offset.into_usize();
        let mut slice = &self[o..];
        match slice.read(buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }

    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let o = offset.into_usize();
        let e = o + buf.len();
        if e > self.len() {
            return Err(String::from("Out of slice").into());
        }
        buf.copy_from_slice(&self[o..e]);
        Ok(())
    }

    fn into_memory(
        self: Rc<Self>,
        offset: Offset,
        size: usize,
    ) -> Result<(Rc<dyn Source>, Offset, End)> {
        assert!(offset.into_usize() + size <= self.len());
        Ok((self, offset, End::new_size(size as u64)))
    }

    fn get_slice(&self, offset: Offset, end: Offset) -> Result<&[u8]> {
        assert!(offset <= end);
        assert!(end.into_usize() <= self.len());
        Ok(&self[offset.into_usize()..end.into_usize()])
    }
}
