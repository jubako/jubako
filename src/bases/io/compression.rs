use crate::bases::*;
use primitive::*;
use std::io::{BorrowedBuf, Read};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{spawn, yield_now};

// A intermediate object acting as source for ReaderWrapper and FluxWrapper.
// It wrapper a Read object (a decoder) and decode in a internal buffer.
// It allow implementation of Reader and Flux.
pub struct SeekableDecoder {
    buffer: Arc<RwLock<Vec<u8>>>,
    decoded: Arc<AtomicUsize>,
}

fn decode_to_end<T: Read + Send>(
    buffer: Arc<RwLock<Vec<u8>>>,
    decoded: Arc<AtomicUsize>,
    mut decoder: T,
    chunk_size: usize,
) -> Result<()> {
    let mut uncompressed = 0;
    let total_size = buffer.read().unwrap().capacity();
    let mut chunk = vec![0; chunk_size];
    let chunk = chunk.as_mut_slice();
    //println!("Decompressing to {total_size}");
    while uncompressed < total_size {
        let size = std::cmp::min(total_size - uncompressed, chunk_size);
        //println!("decompress {size}");
        decoder.read_exact(&mut chunk[0..size])?;
        uncompressed += size;
        {
            let mut buffer = buffer.write().unwrap();
            let uninit = buffer.spare_capacity_mut();
            let mut uninit = BorrowedBuf::from(&mut uninit[0..size]);
            uninit.unfilled().append(&chunk[0..size]);
            unsafe {
                buffer.set_len(uncompressed);
            };
        }
        decoded.store(uncompressed, Ordering::SeqCst);
        yield_now();
    }
    Ok(())
}

impl SeekableDecoder {
    pub fn new<T: Read + Send + 'static>(decoder: T, size: Size) -> Self {
        let buffer = Arc::new(RwLock::new(Vec::with_capacity(size.into_usize())));
        let write_buffer = Arc::clone(&buffer);
        let decoded = Arc::new(AtomicUsize::new(0));
        let write_decoded = Arc::clone(&decoded);
        spawn(move || {
            decode_to_end(write_buffer, write_decoded, decoder, 4 * 1024).unwrap();
        });
        Self { buffer, decoded }
    }
    /*
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
    */

    #[inline]
    pub fn decode_to(&self, end: Offset) {
        let end = end.into_usize();
        while end > self.decoded() {
            //println!("Have to wait {end} > {decoded}");
            yield_now();
        }
    }

    #[inline]
    pub fn decoded(&self) -> usize {
        self.decoded.load(Ordering::SeqCst)
    }

    pub fn decoded_slice(&self) -> &[u8] {
        let buffer = self.buffer.read().unwrap();
        let ptr = buffer.as_ptr();
        let size = buffer.len();
        unsafe { std::slice::from_raw_parts(ptr, size) }
    }
}

impl Source for SeekableDecoder {
    fn size(&self) -> Size {
        self.buffer.read().unwrap().capacity().into()
    }
    fn read(&self, offset: Offset, buf: &mut [u8]) -> Result<usize> {
        let end = offset + buf.len();
        self.decode_to(end);
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
        self.decode_to(end);
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
        self.decode_to(offset + size);
        Ok((self, offset, End::new_size(size as u64)))
    }

    fn into_memory_source(
        self: Arc<Self>,
        offset: Offset,
        size: usize,
    ) -> Result<(Arc<dyn MemorySource>, Offset, End)> {
        debug_assert!((offset + size).is_valid(self.size()));
        self.decode_to(offset + size);
        Ok((self, offset, End::new_size(size as u64)))
    }

    fn read_u8(&self, offset: Offset) -> Result<u8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_u8(slice))
    }

    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_u16(slice))
    }

    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_u32(slice))
    }

    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_u64(slice))
    }

    fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_to_u64(size as usize, slice))
    }

    fn read_i8(&self, offset: Offset) -> Result<i8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_i8(slice))
    }

    fn read_i16(&self, offset: Offset) -> Result<i16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_i16(slice))
    }

    fn read_i32(&self, offset: Offset) -> Result<i32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_i32(slice))
    }

    fn read_i64(&self, offset: Offset) -> Result<i64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_i64(slice))
    }

    fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(read_to_i64(size as usize, slice))
    }
}

impl MemorySource for SeekableDecoder {
    fn get_slice(&self, offset: Offset, end: Offset) -> Result<&[u8]> {
        debug_assert!(offset <= end);
        debug_assert!(end.is_valid(self.size()));
        self.decode_to(end);
        Ok(&self.decoded_slice()[offset.into_usize()..end.into_usize()])
    }
}

/*
#[cfg(feature = "lz4")]
pub type Lz4Source<T> = SeekableDecoder<lz4::Decoder<T>>;

#[cfg(feature = "lzma")]
pub type LzmaSource<T> = SeekableDecoder<lzma::LzmaReader<T>>;

#[cfg(feature = "zstd")]
pub type ZstdSource<'a, T> = SeekableDecoder<zstd::Decoder<'a, T>>;
*/
