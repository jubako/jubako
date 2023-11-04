use crate::bases::*;
use std::io::Read;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::spawn;
use zerocopy::byteorder::{ByteOrder, LittleEndian as LE};

/*
SyncVec is mostly a Arc<Vec<u8>> where the only protected part is its length
(decoded).
We access its data throw a local Vec<u8> pointing to the share data.
The data itself is accessed without race protection.
This is valid as we have:
- Only one writer writting only after decoded (data.length())
- Several reader reading only before decoded.
- No reallocation

We don't need to sync access modification to data as it is local only.

As we don't want the Vec<u8> to be dropped as we are writting in it,
we must create it through a Arc<Vec<u8>>.

At SyncVec drop, we recreate the Arc and let it being drop.
We don't have to change the length of the store `_arc`.
When drop, Vec will free the whole allocated buffer.
Individual elements of the vec don't have to be deallocated as they are u8.
*/

struct SyncVecWr {
    _arc: Arc<Vec<u8>>,
    data: ManuallyDrop<Vec<u8>>,
    total_size: usize,
    decoded: Arc<(Mutex<usize>, Condvar)>,
}

unsafe impl Send for SyncVecWr {}

struct SyncVecRd {
    _arc: Arc<Vec<u8>>,
    buffer: *const u8,
    total_size: usize,
    decoded: Arc<(Mutex<usize>, Condvar)>,
}

unsafe impl Send for SyncVecRd {}
unsafe impl Sync for SyncVecRd {}

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
        unsafe { std::slice::from_raw_parts(self.buffer, size) }
    }
}

fn create_sync_vec(size: usize) -> (SyncVecWr, SyncVecRd) {
    let buffer = Arc::new(Vec::with_capacity(size));
    let decoded = Arc::new((Mutex::new(0), Condvar::new()));
    let buffer_ptr = buffer.as_ptr();
    let rd = SyncVecRd {
        _arc: Arc::clone(&buffer),
        buffer: buffer_ptr,
        total_size: size,
        decoded: Arc::clone(&decoded),
    };
    let rw = SyncVecWr {
        _arc: buffer,
        data: ManuallyDrop::new(unsafe { Vec::from_raw_parts(buffer_ptr as *mut u8, 0, size) }),
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
    mut buffer: SyncVecWr,
    chunk_size: usize,
) -> Result<()> {
    let total_size = buffer.total_size;
    let mut uncompressed = 0;
    //println!("Decompressing to {total_size}");
    while uncompressed < total_size {
        let size = std::cmp::min(total_size - uncompressed, chunk_size);
        //  println!("decompress {size}");

        uncompressed += decoder
            .by_ref()
            .take(size as u64)
            .read_to_end(&mut buffer.data)?;
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
        Ok(slice[0])
    }

    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_u16(slice))
    }

    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_u32(slice))
    }

    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_u64(slice))
    }

    fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_uint(slice, size as usize))
    }

    fn read_i8(&self, offset: Offset) -> Result<i8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(slice[0] as i8)
    }

    fn read_i16(&self, offset: Offset) -> Result<i16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_i16(slice))
    }

    fn read_i32(&self, offset: Offset) -> Result<i32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_i32(slice))
    }

    fn read_i64(&self, offset: Offset) -> Result<i64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_i64(slice))
    }

    fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        self.decode_to(end);
        let slice = &self.decoded_slice()[offset.into_usize()..end.into_usize()];
        Ok(LE::read_int(slice, size as usize))
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

    fn into_source(self: Arc<Self>) -> Arc<dyn Source> {
        self
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
