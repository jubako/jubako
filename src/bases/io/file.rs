use crate::bases::*;
use memmap2::{Advice, MmapOptions};
use std::fs::File;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::os::unix::prelude::AsRawFd;
use std::sync::Arc;
use std::sync::Mutex;
use zerocopy::byteorder::little_endian::{I16, I32, I64, U16, U32, U64};
use zerocopy::byteorder::{ByteOrder, LittleEndian as LE};
use zerocopy::AsBytes;

pub struct FileSource {
    source: Mutex<io::BufReader<File>>,
    len: u64,
}

impl FileSource {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Self::new(std::fs::File::open(path)?)
    }

    pub fn new(mut source: File) -> Result<Self> {
        let len = source.seek(SeekFrom::End(0))?;
        source.seek(SeekFrom::Start(0))?;
        let source = io::BufReader::with_capacity(1024, source);
        Ok(FileSource {
            source: Mutex::new(source),
            len,
        })
    }
}

impl Deref for FileSource {
    type Target = Mutex<io::BufReader<File>>;
    fn deref(&self) -> &Self::Target {
        &self.source
    }
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

    fn into_memory_source(
        self: Arc<Self>,
        region: Region,
    ) -> Result<(Arc<dyn MemorySource>, Region)> {
        if region.size().into_u64() < 1024 {
            let mut f = self.lock().unwrap();
            let mut buf = Vec::with_capacity(region.size().into_usize());
            f.seek(SeekFrom::Start(region.begin().into_u64()))?;
            f.by_ref()
                .take(region.size().into_u64())
                .read_to_end(&mut buf)?;
            Ok((
                Arc::new(buf),
                Region::new_from_size(Offset::zero(), region.size()),
            ))
        } else {
            let mut mmap_options = MmapOptions::new();
            mmap_options
                .offset(region.begin().into_u64())
                .len(region.size().into_usize())
                .populate();
            let mmap =
                unsafe { mmap_options.map(self.source.lock().unwrap().get_ref().as_raw_fd())? };
            mmap.advise(Advice::populate_read())?;
            mmap.advise(Advice::will_need())?;
            Ok((
                Arc::new(mmap),
                Region::new_from_size(Offset::zero(), region.size()),
            ))
        }
    }

    fn read_u8(&self, offset: Offset) -> Result<u8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret: u8 = 0;
        let mut f = self.source.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(ret.as_bytes_mut())?;
        Ok(ret)
    }

    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret = U16::ZERO;
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(ret.as_bytes_mut())?;
        Ok(ret.get())
    }

    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret = U32::ZERO;
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(ret.as_bytes_mut())?;
        Ok(ret.get())
    }

    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret = U64::ZERO;
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(ret.as_bytes_mut())?;
        Ok(ret.get())
    }

    fn read_usized(&self, offset: Offset, size: ByteSize) -> Result<u64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret = U64::ZERO;
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(&mut ret.as_bytes_mut()[..size as usize])?;
        Ok(ret.get())
    }

    fn read_i8(&self, offset: Offset) -> Result<i8> {
        let end = offset + 1;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret: i8 = 0;
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(ret.as_bytes_mut())?;
        Ok(ret)
    }

    fn read_i16(&self, offset: Offset) -> Result<i16> {
        let end = offset + 2;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret = I16::ZERO;
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(ret.as_bytes_mut())?;
        Ok(ret.get())
    }

    fn read_i32(&self, offset: Offset) -> Result<i32> {
        let end = offset + 4;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret = I32::ZERO;
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(ret.as_bytes_mut())?;
        Ok(ret.get())
    }

    fn read_i64(&self, offset: Offset) -> Result<i64> {
        let end = offset + 8;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut ret = I64::ZERO;
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(ret.as_bytes_mut())?;
        Ok(ret.get())
    }

    fn read_isized(&self, offset: Offset, size: ByteSize) -> Result<i64> {
        let end = offset + size as usize;
        if !end.is_valid(self.size()) {
            return Err(format!("Out of slice. {end} ({offset}) > {}", self.size()).into());
        }
        let mut buf = [0u8; 8];
        let mut f = self.lock().unwrap();
        f.seek(SeekFrom::Start(offset.into_u64()))?;
        f.read_exact(&mut buf[..size as usize])?;
        Ok(LE::read_int(&buf, size as usize))
    }
}
