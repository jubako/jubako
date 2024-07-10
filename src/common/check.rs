use crate::bases::*;
use std::cmp;
use std::fmt::Debug;
use std::io::Read;

#[derive(Clone, Copy)]
pub(crate) enum CheckKind {
    None = 0,
    Blake3 = 1,
}

impl CheckKind {
    pub(crate) fn block_size(self) -> Size {
        match self {
            Self::None => BlockCheck::Crc32.size() + 1,
            Self::Blake3 => BlockCheck::Crc32.size() + 33,
        }
    }
}

impl Parsable for CheckKind {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let kind = parser.read_u8()?;
        match kind {
            0_u8 => Ok(CheckKind::None),
            1_u8 => Ok(CheckKind::Blake3),
            _ => Err(format_error!(&format!("Invalid check kind {kind}"), parser)),
        }
    }
}

impl Serializable for CheckKind {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u8(*self as u8)
    }
}

impl Parsable for blake3::Hash {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let mut v = [0_u8; blake3::OUT_LEN];
        parser.read_data(&mut v)?;
        Ok(v.into())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CheckInfo {
    b3hash: Option<blake3::Hash>,
}

impl Parsable for CheckInfo {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let kind = CheckKind::parse(parser)?;
        let b3hash = match kind {
            CheckKind::Blake3 => Some(blake3::Hash::parse(parser)?),
            _ => None,
        };
        Ok(Self { b3hash })
    }
}

impl BlockParsable for CheckInfo {}

impl Serializable for CheckInfo {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        match self.b3hash {
            None => CheckKind::None.serialize(ser),
            Some(hash) => {
                CheckKind::Blake3.serialize(ser)?;
                ser.write_data(hash.as_bytes())?;
                Ok(33)
            }
        }
    }
}

impl CheckInfo {
    pub(crate) fn new_none() -> Self {
        Self { b3hash: None }
    }

    pub(crate) fn new_blake3(source: &mut dyn Read) -> Result<Self> {
        let mut hasher = blake3::Hasher::new();
        hasher.update_reader(source)?;
        let hash = hasher.finalize();
        Ok(Self { b3hash: Some(hash) })
    }

    pub(crate) fn check(&self, source: &mut dyn Read) -> Result<bool> {
        if let Some(b3hash) = self.b3hash {
            let mut hasher = blake3::Hasher::new();
            hasher.update_reader(source)?;
            let hash = hasher.finalize();
            Ok(hash == b3hash)
        } else {
            Ok(true)
        }
    }
}

// Pack info size is pack info + 4 bytes of crc32
const PACK_INFO_SIZE: u64 = super::PackInfo::BLOCK_SIZE as u64;

// The first 38 bytes of the PackInfo must be checked.
const PACK_INFO_TO_CHECK: u64 = 38;

pub(crate) struct ManifestCheckStream<'a, S: Read> {
    source: &'a mut S,
    current_offset: u64,
    pack_offset: u64,
    start_safe_zone: u64,
}

impl<'a, S: Read> ManifestCheckStream<'a, S> {
    pub fn new_from_offset_iter(
        source: &'a mut S,
        mut pack_offsets: impl Iterator<Item = Offset>,
    ) -> Self {
        let (pack_offset, pack_count) = match pack_offsets.next() {
            Some(pack_offset) => {
                let pack_count = pack_offsets.count() + 1;
                (pack_offset, PackCount::from(pack_count as u16))
            }
            None => (Offset::zero(), 0.into()),
        };
        Self::new(source, pack_offset, pack_count)
    }

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
    // Clippy emit a false positive. See https://github.com/rust-lang/rust-clippy/issues/12519
    #[allow(clippy::unused_io_amount)]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // The check stream want to exclude the 218 bytes (PACK_INFO_SIZE-PACK_INFO_TO_CHECK)
        // in all pack_info (location of the pack and CRC32).
        // So data we don't want to check are positionned between
        // pack_offset + k*(256+4) + 38  and 128 + (k+1)*(256+4)
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
