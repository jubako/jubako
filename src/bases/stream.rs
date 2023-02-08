///! All base traits use to produce structure from raw data.
use crate::bases::*;
use std::io::{BorrowedBuf, Read};
use std::sync::Arc;

// A wrapper arount someting to implement Flux trait
#[derive(Debug)]
pub struct Stream {
    source: Arc<dyn Source>,
    origin: Offset,
    end: Offset,
    offset: Offset,
}

impl Stream {
    pub fn new<T: Source + 'static + Sync>(source: T, end: End) -> Self {
        let end = match end {
            End::None => source.size().into(),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        Self {
            source: Arc::new(source),
            origin: Offset::zero(),
            offset: Offset::zero(),
            end,
        }
    }

    pub fn new_from_parts(
        source: Arc<dyn Source>,
        origin: Offset,
        end: Offset,
        offset: Offset,
    ) -> Self {
        Self {
            source,
            origin,
            end,
            offset,
        }
    }

    pub fn as_flux(&self) -> Flux {
        Flux::new_from_parts(&self.source, self.origin, self.end, self.offset)
    }

    pub fn tell(&self) -> Offset {
        (self.offset - self.origin).into()
    }
    pub fn size(&self) -> Size {
        self.end - self.origin
    }
    pub fn seek(&mut self, pos: Offset) {
        self.offset = self.origin + pos;
        assert!(self.offset <= self.end);
    }
    pub fn reset(&mut self) {
        self.seek(Offset::zero())
    }
    pub fn skip(&mut self, size: Size) -> Result<()> {
        let new_offset = self.offset + size;
        if new_offset <= self.end {
            self.offset = new_offset;
            Ok(())
        } else {
            Err(format_error!(&format!(
                "Cannot skip at offset {} ({}+{}) after end of stream ({}).",
                new_offset, self.offset, size, self.end
            )))
        }
    }
    pub fn global_offset(&self) -> Offset {
        self.offset
    }
    pub fn read_u8(&mut self) -> Result<u8> {
        let ret = self.source.read_u8(self.offset)?;
        self.offset += 1;
        Ok(ret)
    }
    pub fn read_u16(&mut self) -> Result<u16> {
        let ret = self.source.read_u16(self.offset)?;
        self.offset += 2;
        Ok(ret)
    }
    pub fn read_u32(&mut self) -> Result<u32> {
        let ret = self.source.read_u32(self.offset)?;
        self.offset += 4;
        Ok(ret)
    }
    pub fn read_u64(&mut self) -> Result<u64> {
        let ret = self.source.read_u64(self.offset)?;
        self.offset += 8;
        Ok(ret)
    }
    pub fn read_usized(&mut self, size: ByteSize) -> Result<u64> {
        let ret = self.source.read_usized(self.offset, size)?;
        self.offset += size as usize;
        Ok(ret)
    }

    pub fn read_vec(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut v = Vec::with_capacity(size);
        let mut uninit: BorrowedBuf = v.spare_capacity_mut().into();
        self.read_buf_exact(uninit.unfilled())?;
        unsafe {
            v.set_len(size);
        }
        Ok(v)
    }
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.source.read_exact(self.offset, buf)?;
        self.offset += buf.len();
        Ok(())
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let max_len = std::cmp::min(buf.len(), (self.end - self.offset).into_usize());
        let buf = &mut buf[..max_len];
        match self.source.read(self.offset, buf) {
            Ok(s) => {
                self.offset += s;
                Ok(s)
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}

impl From<Flux<'_>> for Stream {
    fn from(flux: Flux) -> Self {
        flux.to_owned()
    }
}
