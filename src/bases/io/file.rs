use crate::bases::*;
use memmap2::MmapOptions;
use std::fs::File;
use std::io;
use std::io::BorrowedBuf;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::sync::Arc;
use std::sync::Mutex;

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

pub struct FileSource(Mutex<BufferedFile>);

impl FileSource {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Self::new(std::fs::File::open(path)?)
    }

    pub fn new(mut source: File) -> Result<Self> {
        let len = source.seek(SeekFrom::End(0))?;
        source.seek(SeekFrom::Start(0))?;
        let source = BufferedFile::new(source, len);
        Ok(FileSource(Mutex::new(source)))
    }
}

impl Deref for FileSource {
    type Target = Mutex<BufferedFile>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Source for FileSource {
    fn size(&self) -> Size {
        (self.lock().unwrap().len as u64).into()
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
    fn into_memory(
        self: Arc<Self>,
        offset: Offset,
        size: usize,
    ) -> Result<(Arc<dyn Source>, Offset, End)> {
        if size < 1024 {
            let mut f = self.lock().unwrap();
            let mut buf = Vec::with_capacity(size);
            let mut uninit: BorrowedBuf = buf.spare_capacity_mut().into();
            f.seek(SeekFrom::Start(offset.into_u64()))?;
            f.read_buf_exact(uninit.unfilled())?;
            unsafe {
                buf.set_len(size);
            }
            Ok((Arc::new(buf), Offset::zero(), End::None))
        } else {
            let mut mmap_options = MmapOptions::new();
            mmap_options.offset(offset.into_u64()).len(size).populate();
            let mmap = unsafe { mmap_options.map(&self.lock().unwrap().deref())? };
            Ok((Arc::new(mmap), Offset::zero(), End::None))
        }
    }

    fn get_slice(&self, _offset: Offset, _end: Offset) -> Result<&[u8]> {
        panic!("Cannot get a slice from a file reader");
    }
}
