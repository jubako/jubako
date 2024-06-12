use crate::bases::*;
use std::borrow::Cow;
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

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.source.read_exact(self.offset, buf)?;
        self.offset += buf.len();
        Ok(())
    }
}

impl Parser for Flux<'_> {
    fn read_slice(&mut self, size: usize) -> Result<Cow<[u8]>> {
        let mut buf = vec![0; size];
        self.source.read_exact(self.offset, &mut buf)?;
        self.offset += size;
        Ok(Cow::Owned(buf))
    }

    fn read_data(&mut self, buf: &mut [u8]) -> Result<()> {
        self.source.read_exact(self.offset, buf)?;
        self.offset += buf.len();
        Ok(())
    }

    fn skip(&mut self, size: usize) -> Result<()> {
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
    fn global_offset(&self) -> Offset {
        self.offset
    }

    fn tell(&self) -> Offset {
        (self.offset - self.region.begin()).into()
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
