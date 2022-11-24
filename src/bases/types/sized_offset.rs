use crate::bases::*;

#[derive(Debug, Clone, Copy)]
pub struct SizedOffset {
    pub size: Size,
    pub offset: Offset,
}

impl SizedOffset {
    pub fn new(size: Size, offset: Offset) -> Self {
        Self { size, offset }
    }
}

impl SizedProducable for SizedOffset {
    type Size = typenum::U8;
}

impl Producable for SizedOffset {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let data = stream.read_u64()?;
        let offset = Offset(data & 0xFF_FF_FF_FF_FF_FF_u64);
        let size = Size::from(data >> 48);
        Ok(Self::new(size, offset))
    }
}

impl Writable for SizedOffset {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let data: u64 = (self.size.into_u64() << 48) + (self.offset.0 & 0xFF_FF_FF_FF_FF_FF_u64);
        stream.write_u64(data)
    }
}
