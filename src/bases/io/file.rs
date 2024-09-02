use crate::bases::*;
use log::warn;
#[cfg(unix)]
use memmap2::Advice;
use memmap2::MmapOptions;
use std::borrow::Cow;
use std::fs::File;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

pub struct FileSource {
    source: Mutex<io::BufReader<File>>,
    path: std::path::PathBuf,
    len: u64,
}

impl FileSource {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let mut s = Self::new(std::fs::File::open(&path)?)?;
        s.path = path.as_ref().into();
        Ok(s)
    }

    pub fn new(mut source: File) -> Result<Self> {
        let len = source.seek(SeekFrom::End(0))?;
        source.seek(SeekFrom::Start(0))?;
        let source = io::BufReader::with_capacity(1024, source);
        Ok(FileSource {
            source: Mutex::new(source),
            len,
            path: "".into(),
        })
    }
}

impl Deref for FileSource {
    type Target = Mutex<io::BufReader<File>>;
    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

#[cfg(target_pointer_width = "64")]
#[inline]
const fn move_to_memory(_region: Region) -> bool {
    true
}

#[cfg(target_pointer_width = "32")]
#[inline]
fn move_to_memory(region: Region) -> bool {
    region.size() <= Size::new(0xFFFF)
}

impl Source for FileSource {
    fn size(&self) -> Size {
        (self.len).into()
    }
    fn read(&self, offset: Offset, buf: &mut [u8]) -> Result<usize> {
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        match f.read(buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }

    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        match f.read_exact(buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }

    fn get_slice(&self, region: ARegion, block_check: BlockCheck) -> Result<Cow<[u8]>> {
        let mut buf = vec![0; region.size().into_usize() + block_check.size()];
        self.read_exact(region.begin(), &mut buf)?;
        if let BlockCheck::Crc32 = block_check {
            assert_slice_crc(&buf)?;
        }
        buf.truncate(region.size().into_usize());
        Ok(Cow::Owned(buf))
    }

    fn cut(
        self: Arc<Self>,
        region: Region,
        block_check: BlockCheck,
        in_memory: bool,
    ) -> Result<(Arc<dyn Source>, Region)> {
        if !move_to_memory(region) || !in_memory {
            if let BlockCheck::Crc32 = block_check {
                warn!("Check of not memory block is not implemented");
            }
            return Ok((self, region));
        }

        // We know from previous test that region.size() is addressable.
        let full_size = ASize::new(region.size().into_u64() as usize + block_check.size());
        if full_size.into_u64() < 4 * 1024 {
            let mut f = self.lock().unwrap();
            let mut buf = Vec::with_capacity(full_size.into_usize());
            f.seek(SeekFrom::Start(region.begin().into_u64()))?;
            f.by_ref()
                .take(full_size.into_u64())
                .read_to_end(&mut buf)?;
            if let BlockCheck::Crc32 = block_check {
                assert_slice_crc(&buf)?;
            }
            Ok((
                Arc::new(buf),
                Region::new_from_size(Offset::zero(), region.size()),
            ))
        } else {
            let mut mmap_options = MmapOptions::new();
            mmap_options
                .offset(region.begin().into_u64())
                .len(full_size.into_usize())
                .populate();
            let mmap = unsafe { mmap_options.map(self.source.lock().unwrap().get_ref())? };
            #[cfg(unix)]
            mmap.advise(Advice::will_need())?;
            if let BlockCheck::Crc32 = block_check {
                assert_slice_crc(&mmap)?;
            }

            Ok((
                Arc::new(mmap),
                Region::new_from_size(Offset::zero(), region.size()),
            ))
        }
    }

    fn display(&self) -> String {
        format!("File {}", self.path.display())
    }
}
