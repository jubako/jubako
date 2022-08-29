use crate::bases::*;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ContentAddress {
    pub pack_id: Id<u8>,
    pub content_id: Idx<u32>,
}

impl ContentAddress {
    pub fn new(pack_id: Id<u8>, content_id: Idx<u32>) -> Self {
        Self {
            pack_id,
            content_id,
        }
    }
}

impl Producable for ContentAddress {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let pack_id = stream.read_u8()?;
        let content_id = stream.read_sized(3)? as u32;
        Ok(ContentAddress {
            pack_id: pack_id.into(),
            content_id: content_id.into(),
        })
    }
}

impl Writable for ContentAddress {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let data: u32 = (self.pack_id.0 as u32) << 24 | (self.content_id.0 & 0x00FFFFFF);
        stream.write_u32(data)
    }
}
