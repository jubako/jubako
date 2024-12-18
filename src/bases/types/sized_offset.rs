use crate::bases::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub struct SizedOffset {
    pub(crate) size: ASize,
    pub(crate) offset: Offset,
}

impl SizedOffset {
    pub(crate) fn new(size: ASize, offset: Offset) -> Self {
        debug_assert!(size.into_u64() <= 0xFF_FF_u64);
        debug_assert!(offset.into_u64() <= 0xFF_FF_FF_FF_FF_FF_u64);
        Self { size, offset }
    }

    pub fn is_zero(&self) -> bool {
        self.size.is_zero() && self.offset.is_zero()
    }
}

impl SizedParsable for SizedOffset {
    const SIZE: usize = 8;
}

impl Parsable for SizedOffset {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let data = parser.read_u64()?;
        let size = ASize::from((data & 0xFF_FF_u64) as usize);
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

#[cfg(feature = "explorable")]
impl graphex::Display for SizedOffset {
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        out.write_str(&format!(
            "{} bytes at offset {}",
            self.size.into_u64(),
            self.offset.into_u64()
        ))
    }
}
