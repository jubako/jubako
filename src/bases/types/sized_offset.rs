use crate::bases::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

    pub fn is_zero(&self) -> bool {
        self.size.is_zero() && self.offset.is_zero()
    }
}

impl SizedProducable for SizedOffset {
    const SIZE: usize = 8;
}

impl Producable for SizedOffset {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let data = flux.read_u64()?;
        let size = Size::from(data & 0xFF_FF_u64);
        let offset = Offset::from(data >> 16);
        Ok(Self::new(size, offset))
    }
}

impl Serializable for SizedOffset {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let data: u64 = (self.offset.into_u64() << 16) + (self.size.into_u64() & 0xFF_FF_u64);
        ser.write_u64(data)
    }
}
