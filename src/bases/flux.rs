use crate::bases::*;
use std::io::Read;
use std::sync::Arc;

// A wrapper arount someting to implement Flux trait
pub struct Flux<'s> {
    pub(crate) source: &'s Arc<dyn Source>,
    pub(crate) region: Region,
    pub(crate) offset: Offset,
}

impl<'s> std::fmt::Debug for Flux<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flux")
            .field("source", self.source)
            .field("region", &self.region)
            .field("offset", &self.offset)
            .finish()
    }
}

impl<'s> Flux<'s> {
    pub fn new_from_parts(source: &'s Arc<dyn Source>, region: Region, offset: Offset) -> Self {
        Self {
            source,
            region,
            offset,
        }
    }

    pub fn tell(&self) -> Offset {
        (self.offset - self.region.begin()).into()
    }
    pub fn size(&self) -> Size {
        self.region.size()
    }
    pub fn seek(&mut self, pos: Offset) {
        self.offset = self.region.begin() + pos;
        assert!(self.offset <= self.region.end());
    }
    pub fn reset(&mut self) {
        self.seek(Offset::zero())
    }
    pub fn skip(&mut self, size: Size) -> Result<()> {
        let new_offset = self.offset + size;
        if new_offset <= self.region.end() {
            self.offset = new_offset;
            Ok(())
        } else {
            Err(format_error!(&format!(
                "Cannot skip at offset {} ({}+{}) after end of flux ({}).",
                new_offset,
                self.offset,
                size,
                self.region.end()
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

    pub fn read_i8(&mut self) -> Result<i8> {
        let ret = self.source.read_i8(self.offset)?;
        self.offset += 1;
        Ok(ret)
    }
    pub fn read_i16(&mut self) -> Result<i16> {
        let ret = self.source.read_i16(self.offset)?;
        self.offset += 2;
        Ok(ret)
    }
    pub fn read_i32(&mut self) -> Result<i32> {
        let ret = self.source.read_i32(self.offset)?;
        self.offset += 4;
        Ok(ret)
    }
    pub fn read_i64(&mut self) -> Result<i64> {
        let ret = self.source.read_i64(self.offset)?;
        self.offset += 8;
        Ok(ret)
    }
    pub fn read_isized(&mut self, size: ByteSize) -> Result<i64> {
        let ret = self.source.read_isized(self.offset, size)?;
        self.offset += size as usize;
        Ok(ret)
    }

    pub fn read_vec(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut v = Vec::with_capacity(size);
        self.by_ref().take(size as u64).read_to_end(&mut v)?;
        Ok(v)
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.source.read_exact(self.offset, buf)?;
        self.offset += buf.len();
        Ok(())
    }
}

impl<'s> Read for Flux<'s> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let max_len = std::cmp::min(buf.len(), (self.region.end() - self.offset).into_usize());
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

impl<'s> From<&'s Reader> for Flux<'s> {
    fn from(reader: &'s Reader) -> Self {
        reader.create_flux_all()
    }
}

impl<'s> From<&SubReader<'s>> for Flux<'s> {
    fn from(reader: &SubReader<'s>) -> Self {
        reader.create_flux_all()
    }
}
