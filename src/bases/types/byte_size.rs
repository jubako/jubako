use crate::bases::*;

#[repr(usize)]
#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone, Copy, serde::Serialize)]
pub enum ByteSize {
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

impl Producable for ByteSize {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        Ok((flux.read_u8()? as usize).try_into()?)
    }
}
impl SizedProducable for ByteSize {
    const SIZE: usize = 1;
}
impl Writable for ByteSize {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u8(*self as u8)
    }
}
