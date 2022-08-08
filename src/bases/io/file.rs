use crate::bases::primitive::*;
use crate::bases::*;
use std::cell::RefCell;
use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::rc::Rc;

pub type FileReader = ReaderWrapper<RefCell<File>>;
pub type FileStream = StreamWrapper<RefCell<File>>;

impl FileReader {
    pub fn new(mut source: File, end: End) -> Self {
        let len = source.seek(SeekFrom::End(0)).unwrap();
        let source = Rc::new(RefCell::new(source));
        let end = match end {
            End::None => Offset(len as u64),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        assert!(end.is_valid(len.into()));
        Self {
            source,
            end,
            origin: Offset(0),
        }
    }

    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let mut f = self.source.borrow_mut();
        f.seek(SeekFrom::Start((self.origin + offset).0))?;
        match f.read_exact(buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }
}

impl Reader for FileReader {
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
        assert!(
            end <= self.end,
            "Stream end ({:?}) is after reader end ({:?})",
            end,
            self.end
        );
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
        let mut d = [0_u8; 1];
        self.read_exact(offset, &mut d)?;
        Ok(u8::from_be_bytes(d))
    }
    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let mut d = [0_u8; 2];
        self.read_exact(offset, &mut d)?;
        Ok(u16::from_be_bytes(d))
    }
    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let mut d = [0_u8; 4];
        self.read_exact(offset, &mut d)?;
        Ok(u32::from_be_bytes(d))
    }
    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(offset, &mut d)?;
        Ok(u64::from_be_bytes(d))
    }
    fn read_usized(&self, offset: Offset, size: usize) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(offset, &mut d[8 - size..])?;
        Ok(u64::from_be_bytes(d))
    }

    fn read_i8(&self, offset: Offset) -> Result<i8> {
        let mut d = [0_u8; 1];
        self.read_exact(offset, &mut d)?;
        Ok(i8::from_be_bytes(d))
    }
    fn read_i16(&self, offset: Offset) -> Result<i16> {
        let mut d = [0_u8; 2];
        self.read_exact(offset, &mut d)?;
        Ok(i16::from_be_bytes(d))
    }
    fn read_i32(&self, offset: Offset) -> Result<i32> {
        let mut d = [0_u8; 4];
        self.read_exact(offset, &mut d)?;
        Ok(i32::from_be_bytes(d))
    }
    fn read_i64(&self, offset: Offset) -> Result<i64> {
        let mut d = [0_u8; 8];
        self.read_exact(offset, &mut d)?;
        Ok(i64::from_be_bytes(d))
    }
    fn read_isized(&self, offset: Offset, size: usize) -> Result<i64> {
        let mut d = [0_u8; 8];
        self.read_exact(offset, &mut d[..size])?;
        Ok(read_to_i64(size, &d))
    }
}

impl FileStream {
    pub fn new(mut source: File, end: End) -> Self {
        let len = source.seek(SeekFrom::End(0)).unwrap();
        let source = Rc::new(RefCell::new(source));
        let end = match end {
            End::None => Offset(len as u64),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        assert!(end.is_valid(len.into()));
        Self {
            source,
            origin: Offset(0),
            end,
            offset: Offset(0),
        }
    }
}

impl Read for FileStream {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let mut file = self.source.as_ref().borrow_mut();
        file.seek(SeekFrom::Start(self.offset.0))?;
        let max_read_size = min(buf.len(), (self.end.0 - self.offset.0) as usize);
        match file.read(&mut buf[..max_read_size]) {
            Ok(s) => {
                self.offset += s;
                Ok(s)
            }
            err => err,
        }
    }
}
