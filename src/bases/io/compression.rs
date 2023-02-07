use crate::bases::*;
use std::io::{BorrowedBuf, Read};
use std::sync::{Arc, Mutex, RwLock};

// A intermediate object acting as source for ReaderWrapper and FluxWrapper.
// It wrapper a Read object (a decoder) and decode in a internal buffer.
// It allow implementation of Reader and Flux.
pub struct SeekableDecoder<T> {
    decoder: Mutex<T>,
    buffer: RwLock<Vec<u8>>,
}

impl<T: Read + Send> SeekableDecoder<T> {
    pub fn new(decoder: T, size: Size) -> Self {
        let buffer = Vec::with_capacity(size.into_usize());
        Self {
            decoder: Mutex::new(decoder),
            buffer: RwLock::new(buffer),
        }
    }

    pub fn decode_to(&self, end: Offset) -> std::result::Result<(), std::io::Error> {
        let mut buffer = self.buffer.write().unwrap();
        if end.into_usize() >= buffer.len() {
            let e = std::cmp::min(end.into_usize(), buffer.capacity());
            let s = e - buffer.len();
            let uninit = buffer.spare_capacity_mut();
            let mut uninit = BorrowedBuf::from(&mut uninit[0..s]);
            self.decoder
                .lock()
                .unwrap()
                .read_buf_exact(uninit.unfilled())?;
            unsafe {
                buffer.set_len(e);
            };
        }
        Ok(())
    }

    pub fn decoded_slice(&self) -> &[u8] {
        let buffer = self.buffer.read().unwrap();
        let ptr = buffer.as_ptr();
        let size = buffer.len();
        unsafe { std::slice::from_raw_parts(ptr, size) }
    }
}

impl<T: Read + 'static + Send> Source for SeekableDecoder<T> {
    fn size(&self) -> Size {
        self.buffer.read().unwrap().capacity().into()
    }
    fn read(&self, offset: Offset, buf: &mut [u8]) -> Result<usize> {
        let end = offset + buf.len();
        self.decode_to(end)?;
        let mut slice = &self.decoded_slice()[offset.into_usize()..];
        match Read::read(&mut slice, buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }
    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let end = offset + buf.len();
        let o = offset.into_usize();
        let e = end.into_usize();
        self.decode_to(end)?;
        let slice = self.decoded_slice();
        if e > slice.len() {
            return Err(String::from("Out of slice").into());
        }
        buf.copy_from_slice(&self.decoded_slice()[o..e]);
        Ok(())
    }

    fn into_memory(
        self: Arc<Self>,
        offset: Offset,
        size: usize,
    ) -> Result<(Arc<dyn Source>, Offset, End)> {
        debug_assert!((offset + size).is_valid(self.size()));
        self.decode_to(offset + size)?;
        Ok((self, offset, End::new_size(size as u64)))
    }

    fn get_slice(&self, offset: Offset, end: Offset) -> Result<&[u8]> {
        debug_assert!(offset <= end);
        debug_assert!(end.is_valid(self.size()));
        self.decode_to(end)?;
        Ok(&self.decoded_slice()[offset.into_usize()..end.into_usize()])
    }
}

#[cfg(feature = "lz4")]
pub type Lz4Source<T> = SeekableDecoder<lz4::Decoder<T>>;

#[cfg(feature = "lzma")]
pub type LzmaSource<T> = SeekableDecoder<lzma::LzmaReader<T>>;

#[cfg(feature = "zstd")]
pub type ZstdSource<'a, T> = SeekableDecoder<zstd::Decoder<'a, T>>;
