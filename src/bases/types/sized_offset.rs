use crate::bases::*;

#[derive(Debug, Clone, Copy)]
pub struct SizedOffset {
    pub size: Size,
    pub offset: Offset,
}

impl SizedOffset {
    pub fn new(size: Size, offset: Offset) -> Self {
        debug_assert!(size.into_u64() <= 0xFF_FF_u64);
        debug_assert!(offset.into_u64() <= 0xFF_FF_FF_FF_FF_FF_u64);
        Self { size, offset }
    }
}

impl Default for SizedOffset {
    fn default() -> Self {
        Self {
            size: Size::zero(),
            offset: Offset::zero(),
        }
    }
}

impl SizedProducable for SizedOffset {
    type Size = typenum::U8;
}

impl Producable for SizedOffset {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let data = flux.read_u64()?;
        let offset = Offset::from(data & 0xFF_FF_FF_FF_FF_FF_u64);
        let size = Size::from(data >> 48);
        Ok(Self::new(size, offset))
    }
}

impl Writable for SizedOffset {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let data: u64 =
            (self.size.into_u64() << 48) + (self.offset.into_u64() & 0xFF_FF_FF_FF_FF_FF_u64);
        stream.write_u64(data)
    }
}
