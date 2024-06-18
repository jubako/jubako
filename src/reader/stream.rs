use crate::bases::*;
use std::io::Read;
use std::sync::Arc;

use super::ByteRegion;

/// A `Read` struct on top of bytes contained in Jubako
///
/// A `Stream` allow to read from a [ByteRegion].
#[derive(Debug)]
pub struct Stream {
    source: Arc<dyn Source>,
    region: Region,
    offset: Offset,
}

impl Stream {
    pub(crate) fn new_from_parts(source: Arc<dyn Source>, region: Region, offset: Offset) -> Self {
        Self {
            source,
            region,
            offset,
        }
    }
}

impl Read for Stream {
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

impl From<Flux<'_>> for Stream {
    fn from(flux: Flux) -> Self {
        Self::new_from_parts(Arc::clone(flux.source), flux.region, flux.offset)
    }
}

impl From<ByteRegion> for Stream {
    fn from(bregion: ByteRegion) -> Self {
        Self::new_from_parts(bregion.source, bregion.region, Offset::zero())
    }
}
