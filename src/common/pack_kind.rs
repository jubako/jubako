use crate::bases::*;
use std::fmt::Debug;

const JBK_MAGIC: [u8; 3] = *b"jbk";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize))]
pub enum PackKind {
    Manifest = b'm',
    Directory = b'd',
    Content = b'c',
    Container = b'C',
}

impl Parsable for PackKind {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        match parser.read_u8()? {
            b'm' => Ok(PackKind::Manifest),
            b'd' => Ok(PackKind::Directory),
            b'c' => Ok(PackKind::Content),
            b'C' => Ok(PackKind::Container),
            kind => Err(format_error!(&format!("Invalid pack kind {kind}"), parser)),
        }
    }
}
impl SizedParsable for PackKind {
    const SIZE: usize = 1;
}

impl Serializable for PackKind {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u8(*self as u8)
    }
}

pub(crate) struct FullPackKind(pub PackKind);

impl Parsable for FullPackKind {
    type Output = PackKind;
    fn parse(parser: &mut impl Parser) -> Result<Self::Output> {
        let mut magic = [0; 3];
        parser.read_data(&mut magic)?;
        if magic != JBK_MAGIC {
            Err(format_error!("Not a JBK kind", parser))
        } else {
            match parser.read_u8()? {
                b'm' => Ok(PackKind::Manifest),
                b'd' => Ok(PackKind::Directory),
                b'c' => Ok(PackKind::Content),
                b'C' => Ok(PackKind::Container),
                kind => Err(format_error!(&format!("Invalid pack kind {kind}"), parser)),
            }
        }
    }
}

impl SizedParsable for FullPackKind {
    const SIZE: usize = 4;
}

impl Serializable for FullPackKind {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += ser.write_data(b"jbk")?;
        written += self.0.serialize(ser)?;
        Ok(written)
    }
}
