use crate::bases::primitive::*;
use crate::bases::*;
use memmap2::Mmap;
use std::io::Read;
use std::rc::Rc;

pub type MmapReader = ReaderWrapper<Mmap>;
pub type MmapStream = StreamWrapper<Mmap>;

impl MmapReader {
    pub fn new(source: Rc<Mmap>, origin: Offset, end: End) -> Self {
        let end = match end {
            End::None => Offset(source.len() as u64),
            End::Offset(o) => o,
            End::Size(s) => origin + s,
        };
        assert!(end.is_valid(source.len().into()));
        Self {
            source,
            end,
            origin,
        }
    }

    fn slice(&self) -> &[u8] {
        let o = self.origin.0 as usize;
        let e = self.end.0 as usize;
        &self.source[o..e]
    }
}

impl Reader for MmapReader {
    fn size(&self) -> Size {
        self.end - self.origin
    }

    fn create_stream(&self, offset: Offset, end: End) -> Box<dyn Stream> {
        let source = Rc::clone(&self.source);
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        assert!(end <= self.end);
        Box::new(StreamWrapper::new_from_parts(source, origin, end, origin))
    }

    fn create_sub_reader(&self, offset: Offset, end: End) -> Box<dyn Reader> {
        let source = Rc::clone(&self.source);
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        assert!(end <= self.end);
        Box::new(ReaderWrapper {
            source,
            origin,
            end,
        })
    }

    fn read_u8(&self, offset: Offset) -> Result<u8> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + 1 > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_u8(&slice[o..]))
    }
    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + 2 > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_u16(&slice[o..]))
    }
    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + 4 > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_u32(&slice[o..]))
    }
    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + 8 > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_u64(&slice[o..]))
    }
    fn read_usized(&self, offset: Offset, size: usize) -> Result<u64> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + size > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_to_u64(size, &slice[o..]))
    }

    fn read_i8(&self, offset: Offset) -> Result<i8> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + 1 > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_i8(&slice[o..]))
    }
    fn read_i16(&self, offset: Offset) -> Result<i16> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + 2 > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_i16(&slice[o..]))
    }
    fn read_i32(&self, offset: Offset) -> Result<i32> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + 4 > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_i32(&slice[o..]))
    }
    fn read_i64(&self, offset: Offset) -> Result<i64> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + 8 > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_i64(&slice[o..]))
    }
    fn read_isized(&self, offset: Offset, size: usize) -> Result<i64> {
        let o = offset.0 as usize;
        let slice = self.slice();
        if o + size > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        Ok(read_to_i64(size, &slice[o..]))
    }
}

impl MmapStream {
    fn slice(&self) -> &[u8] {
        let offset = self.offset.0 as usize;
        let end = self.end.0 as usize;
        &self.source[offset..end]
    }
}

impl Read for MmapStream {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let mut slice = self.slice();
        match slice.read(buf) {
            Ok(s) => {
                self.offset += s;
                Ok(s)
            }
            err => err,
        }
    }
}
