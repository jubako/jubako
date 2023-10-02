use crate::bases::*;
use std::cmp;
use std::fmt::Debug;
use std::io::{self, Read};

#[derive(Clone, Copy)]
pub enum CheckKind {
    None = 0,
    Blake3 = 1,
}

impl Producable for CheckKind {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let kind = flux.read_u8()?;
        match kind {
            0_u8 => Ok(CheckKind::None),
            1_u8 => Ok(CheckKind::Blake3),
            _ => Err(format_error!(&format!("Invalid check kind {kind}"), flux)),
        }
    }
}

impl Writable for CheckKind {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u8(*self as u8)
    }
}

impl Producable for blake3::Hash {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let mut v = [0_u8; blake3::OUT_LEN];
        flux.read_exact(&mut v)?;
        Ok(v.into())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CheckInfo {
    b3hash: Option<blake3::Hash>,
}

impl Producable for CheckInfo {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let kind = CheckKind::produce(flux)?;
        let b3hash = match kind {
            CheckKind::Blake3 => Some(blake3::Hash::produce(flux)?),
            _ => None,
        };
        Ok(Self { b3hash })
    }
}

impl Writable for CheckInfo {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        match self.b3hash {
            None => CheckKind::None.write(stream),
            Some(hash) => {
                CheckKind::Blake3.write(stream)?;
                stream.write_all(hash.as_bytes())?;
                Ok(33)
            }
        }
    }
}

impl CheckInfo {
    pub fn new_blake3(source: &mut dyn Read) -> Result<Self> {
        let mut hasher = blake3::Hasher::new();
        io::copy(source, &mut hasher)?;
        let hash = hasher.finalize();
        Ok(Self { b3hash: Some(hash) })
    }

    pub fn check(&self, source: &mut dyn Read) -> Result<bool> {
        if let Some(b3hash) = self.b3hash {
            let mut hasher = blake3::Hasher::new();
            io::copy(source, &mut hasher)?;
            let hash = hasher.finalize();
            Ok(hash == b3hash)
        } else {
            Ok(true)
        }
    }

    pub fn size(&self) -> Size {
        match self.b3hash {
            None => Size::new(1),
            Some(_) => Size::new(33),
        }
    }
}

const PACK_INFO_SIZE: u64 = super::PackInfo::SIZE as u64;
const PACK_INFO_TO_CHECK: u64 = 38;

pub struct ManifestCheckStream<'a, S: Read> {
    source: &'a mut S,
    current_offset: u64,
    pack_offset: u64,
    start_safe_zone: u64,
}

impl<'a, S: Read> ManifestCheckStream<'a, S> {
    pub fn new(source: &'a mut S, pack_offset: Offset, pack_count: PackCount) -> Self {
        let pack_offset = pack_offset.into_u64();
        let start_safe_zone = pack_offset + pack_count.into_u64() * PACK_INFO_SIZE;
        Self {
            source,
            current_offset: 0,
            pack_offset,
            start_safe_zone,
        }
    }
}

impl<S: Read> Read for ManifestCheckStream<'_, S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // The check stream want to exclude the 218 bytes (PACK_INFO_SIZE-PACK_INFO_TO_CHECK)
        // in all pack_info locating the pack.
        // So data we don't want to check are positionned between
        // pack_offset + k*256 + 38  and 128 + (k+1)*256
        // for k < pack_count
        let offset = self.current_offset;
        let read_size = if offset < self.pack_offset {
            let size = cmp::min(buf.len(), (self.pack_offset - offset) as usize);
            self.source.read(&mut buf[..size])?
        } else if offset >= self.start_safe_zone {
            self.source.read(buf)?
        } else {
            let local_offset = (offset - self.pack_offset) % PACK_INFO_SIZE;
            if local_offset < PACK_INFO_TO_CHECK {
                let size = cmp::min(buf.len(), (PACK_INFO_TO_CHECK - local_offset) as usize);
                self.source.read(&mut buf[..size])?
            } else {
                let size = cmp::min(buf.len(), (PACK_INFO_SIZE - local_offset) as usize);
                let size = self.source.read(&mut buf[..size])?;
                buf[..size].fill(0);
                size
            }
        };
        self.current_offset += read_size as u64;
        Ok(read_size)
    }
}
