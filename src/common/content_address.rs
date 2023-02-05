use crate::bases::*;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct ContentAddress {
    pub pack_id: PackId,
    pub content_id: ContentIdx,
}

impl ContentAddress {
    pub fn new(pack_id: PackId, content_id: ContentIdx) -> Self {
        Self {
            pack_id,
            content_id,
        }
    }
}

impl Producable for ContentAddress {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let pack_id = stream.read_u8()?;
        let content_id = stream.read_sized(ByteSize::U3)? as u32;
        Ok(ContentAddress {
            pack_id: pack_id.into(),
            content_id: content_id.into(),
        })
    }
}

impl Writable for ContentAddress {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let data: u32 = (self.pack_id.into_u32() << 24) | (self.content_id.into_u32() & 0x00FFFFFF);
        stream.write_u32(data)
    }
}
