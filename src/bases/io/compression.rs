use crate::bases::*;
use primitive::*;
use std::io::{BorrowedBuf, Read};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{spawn, yield_now};

/*
SyncVec is mostly a Arc<Vec<u8>> where the only protected part is its length
(decoded).
The data itself is accessed without race protection.
This is valid as we have:
- Only one writer writting only after decoded.
- Several reader reading only before decoded.
- No reallocation

As we don't want the Vec<u8> to be dropped as we are writting in it,
we must create it through a Arc<Vec<u8>>.

At SyncVec drop, we recreate the Arc and let it being drop.
*/

struct SyncVec {
    pub arc_ptr: *const Vec<u8>,
    pub data: *mut u8,
    pub total_size: usize,
    pub decoded: Arc<AtomicUsize>,
}

impl Drop for SyncVec {
    fn drop(&mut self) {
        let _arc = unsafe { Arc::from_raw(self.arc_ptr) };
        // Let rust drop the arc
    }
}

unsafe impl Send for SyncVec {}

// A intermediate object acting as source for ReaderWrapper and FluxWrapper.
// It wrapper a Read object (a decoder) and decode in a internal buffer.
// It allow implementation of Reader and Flux.
pub struct SeekableDecoder {
    buffer: Arc<Vec<u8>>,
    decoded: Arc<AtomicUsize>,
}

fn decode_to_end<T: Read + Send>(mut decoder: T, buffer: SyncVec, chunk_size: usize) -> Result<()> {
    let total_size = buffer.total_size;
    let mut uncompressed = 0;
    //println!("Decompressing to {total_size}");
    while uncompressed < total_size {
        let size = std::cmp::min(total_size - uncompressed, chunk_size);
        //  println!("decompress {size}");

        let slice = std::ptr::slice_from_raw_parts_mut(buffer.data, total_size);
        let uninit_slice = unsafe { slice.as_uninit_slice_mut() }.unwrap();
        let mut uninit = BorrowedBuf::from(&mut uninit_slice[uncompressed..uncompressed + size]);
        decoder.read_buf_exact(uninit.unfilled())?;
        unsafe {
            let mut vec = Vec::from_raw_parts(buffer.data, uncompressed, buffer.total_size);
            vec.set_len(uncompressed + size);
            vec.into_raw_parts();
        }
        uncompressed += size;
        buffer.decoded.store(uncompressed, Ordering::SeqCst);
        yield_now();
    }
    //println!("Decompress done");
    Ok(())
}

impl SeekableDecoder {
    pub fn new<T: Read + Send + 'static>(decoder: T, size: Size) -> Self {
        let buffer = Arc::new(Vec::with_capacity(size.into_usize()));
        let decoded = Arc::new(AtomicUsize::new(0));
        let us = Self {
            buffer: Arc::clone(&buffer),
            decoded: Arc::clone(&decoded),
        };

        // This create a raw *mut [u8] on the allocated buffer
        let buffer_ptr = buffer.as_ptr();
        let ptr = SyncVec {
            arc_ptr: Arc::into_raw(buffer),
            data: buffer_ptr as *mut u8,
            total_size: size.into_usize(),
            decoded,
        };

        spawn(move || {
            decode_to_end(decoder, ptr, 4 * 1024).unwrap();
        });
        us
    }

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
        let size = self.decoded();
        let ptr = self.buffer.as_ptr();
        unsafe { std::slice::from_raw_parts(ptr, size) }
    }
}

impl Source for SeekableDecoder {
    fn size(&self) -> Size {
        self.buffer.capacity().into()
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
