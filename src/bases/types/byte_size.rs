use crate::bases::*;

#[repr(usize)]
#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize))]
pub(crate) enum ByteSize {
    U1 = 1,
    U2,
    U3,
    U4,
    U5,
    U6,
    U7,
    U8,
}

impl TryFrom<usize> for ByteSize {
    type Error = &'static str;

    fn try_from(v: usize) -> std::result::Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::U1),
            2 => Ok(Self::U2),
            3 => Ok(Self::U3),
            4 => Ok(Self::U4),
            5 => Ok(Self::U5),
            6 => Ok(Self::U6),
            7 => Ok(Self::U7),
            8 => Ok(Self::U8),
            _ => Err("Not a valid size of ByteSize"),
        }
    }
}

impl Parsable for ByteSize {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        Ok((parser.read_u8()? as usize).try_into()?)
    }
}
impl SizedParsable for ByteSize {
    const SIZE: usize = 1;
}
impl Serializable for ByteSize {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u8(*self as u8)
    }
}
