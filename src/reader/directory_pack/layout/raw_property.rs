use crate::bases::*;

/// The kind of property definition as specified in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawPropertyKind {
    Padding,
    ContentAddress(u8),
    UnsignedInt,
    SignedInt,
    CharArray,
    PString(bool /*flookup*/, u8 /*idx*/),
    VariantId,
}

/// The property definition as defined in the store.
/// The RawProperty is somehow independent of the other properties :
/// It may be sementically dependent but this structure doesn't represent it.
/// The resolition of the depedencies is made later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawProperty {
    pub size: usize,
    pub kind: RawPropertyKind,
}

impl RawProperty {
    pub fn new(kind: RawPropertyKind, size: usize) -> Self {
        Self { size, kind }
    }
}

impl Producable for RawProperty {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let propinfo = stream.read_u8()?;
        let proptype = propinfo >> 4;
        let propdata = propinfo & 0x0F;
        let (propsize, kind) = match proptype {
            0b0000 => (propdata as u16 + 1, RawPropertyKind::Padding),
            0b0001 => (
                (propdata as u16 + 1) * 4,
                RawPropertyKind::ContentAddress(propdata),
            ),
            0b0010 => (
                (propdata & 0x07) as u16 + 1,
                if (propdata & 0x08) != 0 {
                    RawPropertyKind::SignedInt
                } else {
                    RawPropertyKind::UnsignedInt
                },
            ),
            0b0100 => {
                (
                    if propdata & 0x08 == 0 {
                        (propdata + 1) as u16
                    } else {
                        // We need a complement byte
                        let complement = stream.read_u8()?;
                        (((propdata & 0x03) as u16) << 8) + complement as u16 + 9
                    },
                    RawPropertyKind::CharArray,
                )
            }
            0b0110 | 0b0111 => {
                let flookup: bool = proptype & 0b1 != 0;
                let size = propdata as u16 + 1;
                let keystoreidx = stream.read_u8()?;
                (size, RawPropertyKind::PString(flookup, keystoreidx))
            }
            0b1000 => (1, RawPropertyKind::VariantId),
            _ => {
                return Err(format_error!(
                    &format!("Invalid property type ({})", proptype),
                    stream
                ))
            }
        };
        Ok(Self {
            size: propsize as usize,
            kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(&[0b1000_0000] => RawProperty{size:1, kind:RawPropertyKind::VariantId })]
    #[test_case(&[0b0000_0000] => RawProperty{size:1, kind:RawPropertyKind::Padding })]
    #[test_case(&[0b0000_0111] => RawProperty{size:8, kind:RawPropertyKind::Padding })]
    #[test_case(&[0b0001_0000] => RawProperty{size:4, kind:RawPropertyKind::ContentAddress(0) })]
    #[test_case(&[0b0001_0001] => RawProperty{size:8, kind:RawPropertyKind::ContentAddress(1) })]
    #[test_case(&[0b0001_0011] => RawProperty{size:16, kind:RawPropertyKind::ContentAddress(3) })]
    #[test_case(&[0b0010_0000] => RawProperty{size:1, kind:RawPropertyKind::UnsignedInt })]
    #[test_case(&[0b0010_0010] => RawProperty{size:3, kind:RawPropertyKind::UnsignedInt })]
    #[test_case(&[0b0010_0111] => RawProperty{size:8, kind:RawPropertyKind::UnsignedInt })]
    #[test_case(&[0b0010_1000] => RawProperty{size:1, kind:RawPropertyKind::SignedInt })]
    #[test_case(&[0b0010_1010] => RawProperty{size:3, kind:RawPropertyKind::SignedInt })]
    #[test_case(&[0b0010_1111] => RawProperty{size:8, kind:RawPropertyKind::SignedInt })]
    #[test_case(&[0b0100_0000] => RawProperty{size:1, kind:RawPropertyKind::CharArray })]
    #[test_case(&[0b0100_0111] => RawProperty{size:8, kind:RawPropertyKind::CharArray })]
    #[test_case(&[0b0100_1000, 0x00]=> RawProperty{size:9, kind:RawPropertyKind::CharArray })]
    #[test_case(&[0b0100_1000, 0xFF]=> RawProperty{size:264, kind:RawPropertyKind::CharArray })]
    #[test_case(&[0b0100_1011, 0xFF]=> RawProperty{size:1032, kind:RawPropertyKind::CharArray })]
    #[test_case(&[0b0110_0000, 0x0F]=> RawProperty{size:1, kind:RawPropertyKind::PString(false, 0x0F) })]
    #[test_case(&[0b0110_0111, 0x0F]=> RawProperty{size:8, kind:RawPropertyKind::PString(false, 0x0F) })]
    #[test_case(&[0b0111_0000, 0x0F]=> RawProperty{size:1, kind:RawPropertyKind::PString(true, 0x0F) })]
    #[test_case(&[0b0111_0111, 0x0F]=> RawProperty{size:8, kind:RawPropertyKind::PString(true, 0x0F) })]
    fn test_rawproperty(source: &[u8]) -> RawProperty {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        RawProperty::produce(stream.as_mut()).unwrap()
    }
}
