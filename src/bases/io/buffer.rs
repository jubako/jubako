use crate::bases::*;
use std::io::Read;
use std::sync::Arc;
use zerocopy::byteorder::{ByteOrder, LittleEndian as LE};

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

    fn into_memory_source(
        self: Arc<Self>,
        region: Region,
    ) -> Result<(Arc<dyn MemorySource>, Region)> {
        debug_assert!(region.end().into_usize() <= self.as_ref().as_ref().len());
        Ok((Arc::clone(&(self as Arc<dyn MemorySource>)), region))
    }

    fn read_u8(&self, offset: Offset) -> Result<u8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(slice[0])
    }

    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_u16(slice))
    }

    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_u32(slice))
    }

    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_u64(slice))
    }

    fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_uint(slice, size as usize))
    }

    fn read_i8(&self, offset: Offset) -> Result<i8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(slice[0] as i8)
    }

    fn read_i16(&self, offset: Offset) -> Result<i16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_i16(slice))
    }

    fn read_i32(&self, offset: Offset) -> Result<i32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_i32(slice))
    }

    fn read_i64(&self, offset: Offset) -> Result<i64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_i64(slice))
    }

    fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let slice = &self.as_ref()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_int(slice, size as usize))
    }
}

impl<T: AsRef<[u8]> + 'static + Sync + Send> MemorySource for T {
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
