use crate::bases::*;
use primitive::*;
use std::io::{BorrowedBuf, Read};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::spawn;

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

struct SyncVecWr {
    arc_ptr: *const Vec<u8>,
    data: *mut u8,
    total_size: usize,
    decoded: Arc<(Mutex<usize>, Condvar)>,
}

impl Drop for SyncVecWr {
    fn drop(&mut self) {
        let _arc = unsafe { Arc::from_raw(self.arc_ptr) };
        // Let rust drop the arc
    }
}

unsafe impl Send for SyncVecWr {}

struct SyncVecRd {
    buffer: Arc<Vec<u8>>,
    total_size: usize,
    decoded: Arc<(Mutex<usize>, Condvar)>,
}

impl SyncVecRd {
    #[inline]
    pub fn wait_while<F>(&self, function: F) -> usize
    where
        F: Fn(&mut usize) -> bool,
    {
        let (lock, cvar) = &*self.decoded;
        let decoded = cvar.wait_while(lock.lock().unwrap(), function).unwrap();
        *decoded
    }

    #[inline]
    pub fn current_size(&self) -> usize {
        let (lock, _cvar) = &*self.decoded;
        *lock.lock().unwrap()
    }

    #[inline]
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    #[inline]
    fn slice(&self) -> &[u8] {
        let size = self.current_size();
        let ptr = self.buffer.as_ptr();
        unsafe { std::slice::from_raw_parts(ptr, size) }
    }
}

fn create_sync_vec(size: usize) -> (SyncVecWr, SyncVecRd) {
    let buffer = Arc::new(Vec::with_capacity(size));
    let decoded = Arc::new((Mutex::new(0), Condvar::new()));
    let rd = SyncVecRd {
        buffer: Arc::clone(&buffer),
        total_size: size,
        decoded: Arc::clone(&decoded),
    };
    let buffer_ptr = buffer.as_ptr();
    let rw = SyncVecWr {
        arc_ptr: Arc::into_raw(buffer),
        data: buffer_ptr as *mut u8,
        total_size: size,
        decoded,
    };
    (rw, rd)
}

// A intermediate object acting as source for ReaderWrapper and FluxWrapper.
// It wrapper a Read object (a decoder) and decode in a internal buffer.
// It allow implementation of Reader and Flux.
pub struct SeekableDecoder {
    buffer: SyncVecRd,
}

fn decode_to_end<T: Read + Send>(
    mut decoder: T,
    buffer: SyncVecWr,
    chunk_size: usize,
) -> Result<()> {
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
        let (lock, cvar) = &*buffer.decoded;
        let mut decoded = lock.lock().unwrap();
        *decoded = uncompressed;
        cvar.notify_all();
    }
    //println!("Decompress done");
    Ok(())
}

impl SeekableDecoder {
    pub fn new<T: Read + Send + 'static>(decoder: T, size: Size) -> Self {
        let (write_hand, read_hand) = create_sync_vec(size.into_usize());

        spawn(move || {
            decode_to_end(decoder, write_hand, 4 * 1024).unwrap();
        });
        Self { buffer: read_hand }
    }

    #[inline]
    pub fn decode_to(&self, end: Offset) {
        let end = end.into_usize();
        self.buffer.wait_while(|d: &mut usize| *d < end);
    }

    #[inline]
    pub fn decoded_slice(&self) -> &[u8] {
        self.buffer.slice()
    }
}

impl Source for SeekableDecoder {
    fn size(&self) -> Size {
        self.buffer.total_size().into()
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

    fn into_memory(self: Arc<Self>, region: Region) -> Result<(Arc<dyn Source>, Region)> {
        debug_assert!(region.end().is_valid(self.size()));
        self.decode_to(region.end());
        Ok((self, region))
    }

    fn into_memory_source(
        self: Arc<Self>,
        region: Region,
    ) -> Result<(Arc<dyn MemorySource>, Region)> {
        debug_assert!(region.end().is_valid(self.size()));
        self.decode_to(region.end());
        Ok((self, region))
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
    fn get_slice(&self, region: Region) -> Result<&[u8]> {
        debug_assert!(region.end().is_valid(self.size()));
        self.decode_to(region.end());
        Ok(&self.decoded_slice()[region.begin().into_usize()..region.end().into_usize()])
    }

    unsafe fn get_slice_unchecked(&self, region: Region) -> Result<&[u8]> {
        debug_assert!(region.end().is_valid(self.size()));
        Ok(&self.decoded_slice()[region.begin().into_usize()..region.end().into_usize()])
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
