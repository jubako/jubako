use super::buffer;
use super::mmap::MmapReader;
use crate::bases::primitive::*;
use crate::bases::*;
use memmap2::MmapOptions;
use std::cell::RefCell;
use std::cmp::min;
use std::fs::File;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::rc::Rc;

pub struct BufferedFile {
    source: io::BufReader<File>,
    len: i64,
    pos: i64,
}

impl BufferedFile {
    pub fn new(source: File, len: u64) -> Self {
        Self {
            source: io::BufReader::with_capacity(512, source),
            len: len as i64,
            pos: 0,
        }
    }
}

impl AsRawFd for &BufferedFile {
    fn as_raw_fd(&self) -> RawFd {
        self.source.get_ref().as_raw_fd()
    }
}

impl Read for BufferedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let delta = self.source.read(buf)?;
        self.pos += delta as i64;
        Ok(delta)
    }
}

impl Seek for BufferedFile {
    fn seek(&mut self, pos: SeekFrom) -> std::result::Result<u64, std::io::Error> {
        let delta = match pos {
            SeekFrom::Current(o) => o,
            SeekFrom::Start(s) => s as i64 - self.pos,
            SeekFrom::End(e) => (self.len - e) - self.pos,
        };
        self.source.seek_relative(delta)?;
        self.pos += delta;
        Ok(self.pos as u64)
    }
}

pub type FileReader = ReaderWrapper<RefCell<BufferedFile>>;
pub type FileStream = StreamWrapper<RefCell<BufferedFile>>;

impl FileReader {
    pub fn new(mut source: File, end: End) -> Self {
        let len = source.seek(SeekFrom::End(0)).unwrap();
        source.seek(SeekFrom::Start(0)).unwrap();
        let source = BufferedFile::new(source, len);
        let source = Rc::new(RefCell::new(source));
        let end = match end {
            End::None => Offset::from(len),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        assert!(end.is_valid(len.into()));
        Self {
            source,
            end,
            origin: Offset::zero(),
        }
    }

    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let mut f = self.source.borrow_mut();
        f.seek(SeekFrom::Start((self.origin + offset).into_u64()))?;
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

    fn create_sub_memory_reader(&self, offset: Offset, end: End) -> Result<Box<dyn Reader>> {
        let size = match end {
            End::None => self.end - (self.origin + offset),
            End::Offset(o) => o - offset,
            End::Size(s) => s,
        };
        if size < Size::new(1024) {
            let mut stream = self.create_stream(offset, end);
            let buf = stream.read_vec(size.into_usize())?;
            Ok(Box::new(buffer::BufReader::new(buf, End::None)))
        } else {
            let mut mmap_options = MmapOptions::new();
            mmap_options
                .offset((self.origin + offset).into_u64())
                .len(size.into_usize())
                .populate();
            let mmap = unsafe { mmap_options.map(&self.source.borrow().deref())? };
            Ok(Box::new(MmapReader::new(
                Rc::new(mmap),
                Offset::zero(),
                End::None,
            )))
        }
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
        source.seek(SeekFrom::Start(0)).unwrap();
        let source = BufferedFile::new(source, len);
        let source = Rc::new(RefCell::new(source));
        let end = match end {
            End::None => Offset::from(len),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        assert!(end.is_valid(len.into()));
        Self {
            source,
            origin: Offset::zero(),
            end,
            offset: Offset::zero(),
        }
    }
}

impl Read for FileStream {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let mut file = self.source.as_ref().borrow_mut();
        file.seek(SeekFrom::Start(self.offset.into_u64()))?;
        let max_read_size = min(buf.len(), (self.end - self.offset).into_usize());
        match file.read(&mut buf[..max_read_size]) {
            Ok(s) => {
                self.offset += s;
                Ok(s)
            }
            err => err,
        }
    }
}
