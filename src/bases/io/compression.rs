use crate::bases::primitive::*;
use crate::bases::*;
use std::cell::RefCell;
use std::cmp;
use std::io::{Read, ReadBuf};
use std::rc::Rc;

// A intermediate object acting as source for ReaderWrapper and StreamWrapper.
// It wrapper a Read object (a decoder) and decode in a internal buffer.
// It allow implementation of Reader and Stream.
pub struct SeekableDecoder<T> {
    decoder: RefCell<T>,
    buffer: RefCell<Vec<u8>>,
}

impl<T: Read> SeekableDecoder<T> {
    pub fn new(decoder: T, size: Size) -> Self {
        let buffer = Vec::with_capacity(size.0 as usize);
        Self {
            decoder: RefCell::new(decoder),
            buffer: RefCell::new(buffer),
        }
    }

    pub fn decode_to(&self, end: Offset) -> std::result::Result<(), std::io::Error> {
        let mut buffer = self.buffer.borrow_mut();
        if end.0 >= buffer.len() as u64 {
            let e = std::cmp::min(end.0 as usize, buffer.capacity());
            let s = e - buffer.len();
            let uninit = buffer.spare_capacity_mut();
            self.decoder
                .borrow_mut()
                .read_buf_exact(&mut ReadBuf::uninit(&mut uninit[0..s]))?;
            unsafe {
                buffer.set_len(e);
            };
        }
        Ok(())
    }

    pub fn decoded_slice(&self) -> &[u8] {
        let ptr = self.buffer.borrow().as_ptr();
        let size = self.buffer.borrow().len();
        unsafe { std::slice::from_raw_parts(ptr, size) }
    }
}

pub type Lz4Reader<T> = ReaderWrapper<SeekableDecoder<lz4::Decoder<T>>>;
pub type LzmaReader<T> = ReaderWrapper<SeekableDecoder<lzma::LzmaReader<T>>>;
pub type ZstdReader<'a, T> = ReaderWrapper<SeekableDecoder<zstd::Decoder<'a, T>>>;

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
        let slice = self.source.decoded_slice();
        if e > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        buf.copy_from_slice(&self.source.decoded_slice()[o..e]);
        Ok(())
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

impl<T: Read> StreamWrapper<SeekableDecoder<T>> {
    fn decoded_slice(&self) -> &[u8] {
        let o = self.offset.0 as usize;
        let slice = self.source.decoded_slice();
        let e = cmp::min(self.end.0 as usize, slice.len());
        &self.source.decoded_slice()[o..e]
    }
}

impl<T: Read> Read for StreamWrapper<SeekableDecoder<T>> {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let end = self.offset + buf.len();
        self.source.decode_to(end)?;
        let mut slice = self.decoded_slice();
        match slice.read(buf) {
            Ok(s) => {
                self.offset += s;
                Ok(s)
            }
            err => err,
        }
    }
}
