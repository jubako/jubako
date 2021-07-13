use crate::bases::stream::*;
use crate::bases::types::*;
use crate::io::*;
use crate::primitive::*;
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::rc::Rc;

/// A Reader is the main trait containing stuff from "raw data".
/// A Reader may have a size.
/// It is possible to create stream from a reader.
/// Each reader is independant.
/// Data is never modified.
pub trait Reader {
    fn size(&self) -> Size;

    fn create_stream(&self, offset: Offset, end: End) -> Box<dyn Stream>;
    fn create_sub_reader(&self, offset: Offset, end: End) -> Box<dyn Reader>;

    fn read_u8(&self, offset: Offset) -> Result<u8>;
    fn read_u16(&self, offset: Offset) -> Result<u16>;
    fn read_u32(&self, offset: Offset) -> Result<u32>;
    fn read_u64(&self, offset: Offset) -> Result<u64>;
    fn read_sized(&self, offset: Offset, size: usize) -> Result<u64>;
}

pub struct ReaderWrapper<T> {
    source: Rc<T>,
    origin: Offset,
    end: Offset,
}
pub type BufReader = ReaderWrapper<Vec<u8>>;
pub type FileReader = ReaderWrapper<RefCell<File>>;
pub type Lz4Reader<T> = ReaderWrapper<SeekableDecoder<lz4::Decoder<T>>>;
pub type LzmaReader<T> = ReaderWrapper<SeekableDecoder<lzma::LzmaReader<T>>>;
pub type ZstdReader<'a, T> = ReaderWrapper<SeekableDecoder<zstd::Decoder<'a, T>>>;

impl BufReader {
    pub fn new(source: Vec<u8>, end: End) -> Self {
        let source = Rc::new(source);
        let end = match end {
            End::None => Offset(source.len() as u64),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        assert!(end.is_valid(source.len().into()));
        Self {
            source,
            end,
            origin: Offset(0),
        }
    }
}

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

impl<T: Read> ReaderWrapper<SeekableDecoder<T>> {
    pub fn new(decoder: T, outsize: Size) -> Self {
        let source = Rc::new(SeekableDecoder::new(decoder, outsize));
        let end = outsize.into();
        Self {
            source,
            end,
            origin: Offset(0),
        }
    }
    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let o = self.origin + offset;
        let end = o + buf.len();
        let o = o.0 as usize;
        let e = end.0 as usize;
        self.source.decode_to(end)?;
        buf.copy_from_slice(&self.source.decoded_slice()[o..e]);
        Ok(())
    }
}

impl Reader for BufReader {
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
        Ok(read_u8(&self.source[o..o + 1]))
    }
    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let o = offset.0 as usize;
        Ok(read_u16(&self.source[o..o + 2]))
    }
    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let o = offset.0 as usize;
        Ok(read_u32(&self.source[o..o + 4]))
    }
    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let o = offset.0 as usize;
        Ok(read_u64(&self.source[o..o + 8]))
    }
    fn read_sized(&self, offset: Offset, size: usize) -> Result<u64> {
        let o = offset.0 as usize;
        Ok(read_to_u64(size, &self.source[o..o + size]))
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
    fn read_sized(&self, offset: Offset, size: usize) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(offset, &mut d[8 - size..])?;
        Ok(u64::from_be_bytes(d))
    }
}

impl<T: 'static + Read> Reader for ReaderWrapper<SeekableDecoder<T>> {
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
    fn read_sized(&self, offset: Offset, size: usize) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(offset, &mut d[8 - size..])?;
        Ok(u64::from_be_bytes(d))
    }
}
