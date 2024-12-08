use crate::bases::*;
use std::borrow::Cow;
use std::io::Read;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

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

static DECOMPRESSION_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

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
pub(crate) struct SeekableDecoder {
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
    pub fn new<T: Read + Send + 'static>(decoder: T, size: ASize) -> Self {
        let (write_hand, read_hand) = create_sync_vec(size.into_usize());

        DECOMPRESSION_POOL
            .get_or_init(|| {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(8)
                    .thread_name(|idx| format!("DecompThread{idx}"))
                    .build()
                    .unwrap()
            })
            .spawn(move || {
                decode_to_end(decoder, write_hand, 4 * 1024).unwrap();
            });
        Self { buffer: read_hand }
    }

    #[inline]
    pub fn decode_to(&self, end: usize) {
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
        let end = std::cmp::min(
            offset.force_into_usize() + buf.len(),
            self.buffer.total_size(),
        );
        self.decode_to(end);
        let mut slice = &self.decoded_slice()[offset.force_into_usize()..];
        match Read::read(&mut slice, buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }
    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let o = offset.force_into_usize();
        let end = o + buf.len();
        if end > self.buffer.total_size() {
            return Err(format_error!("Out of slice"));
        }
        self.decode_to(end);
        let slice = self.decoded_slice();
        assert!(end <= slice.len());
        buf.copy_from_slice(&self.decoded_slice()[o..end]);
        Ok(())
    }

    fn get_slice(&self, region: ARegion, block_check: BlockCheck) -> Result<Cow<[u8]>> {
        if let BlockCheck::Crc32 = block_check {
            unreachable!()
        }
        if !region.end().is_valid(self.size()) {
            return Err(format_error!(format!(
                "Out of slice. {} > {}",
                region.end(),
                self.size()
            )));
        }
        self.decode_to(region.end().force_into_usize());
        Ok(Cow::Borrowed(
            &self.decoded_slice()
                [region.begin().force_into_usize()..region.end().force_into_usize()],
        ))
    }

    fn cut(
        self: Arc<Self>,
        region: Region,
        block_check: BlockCheck,
        _in_memory: bool,
    ) -> Result<(Arc<dyn Source>, Region)> {
        debug_assert!(region.end().is_valid(self.size()));
        if let BlockCheck::Crc32 = block_check {
            unreachable!()
        }
        Ok((self, region))
    }

    fn display(&self) -> String {
        "SeekableDecoderStream".into()
    }
}

/*
#[cfg(feature = "lz4")]
pub(crate) type Lz4Source<T> = SeekableDecoder<lz4::Decoder<T>>;

#[cfg(feature = "lzma")]
pub(crate) type LzmaSource<T> = SeekableDecoder<lzma::LzmaReader<T>>;

#[cfg(feature = "zstd")]
pub(crate) type ZstdSource<'a, T> = SeekableDecoder<zstd::Decoder<'a, T>>;
*/
