use crate::bases::*;
use memmap2::MmapOptions;
use primitive::*;
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

    pub fn get_slice(&mut self, offset: Offset, end: Offset) -> Result<&[u8]> {
        use std::io::BufRead;
        self.seek(SeekFrom::Start(offset.into_u64()))?;
        let buf = self.source.fill_buf()?;
        let size = (end - offset).into_usize();
        Ok(&buf[..size])
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

    fn into_memory_source(
        self: Arc<Self>,
        offset: Offset,
        size: usize,
    ) -> Result<(Arc<dyn MemorySource>, Offset, End)> {
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

    fn read_u8(&self, offset: Offset) -> Result<u8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_u8(slice))
    }

    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_u16(slice))
    }

    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_u32(slice))
    }

    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_u64(slice))
    }

    fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_to_u64(size as usize, slice))
    }

    fn read_i8(&self, offset: Offset) -> Result<i8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_i8(slice))
    }

    fn read_i16(&self, offset: Offset) -> Result<i16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_i16(slice))
    }

    fn read_i32(&self, offset: Offset) -> Result<i32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_i32(slice))
    }

    fn read_i64(&self, offset: Offset) -> Result<i64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_i64(slice))
    }

    fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut f = self.lock().unwrap();
        let slice = f.get_slice(offset, end)?;
        Ok(read_to_i64(size as usize, slice))
    }
}
