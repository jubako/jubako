use crate::bases::*;

/// The kind of key definition as specified in the store.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum KeyDefKind {
    Padding,
    ContentAddress(bool),
    UnsignedInt,
    SignedInt,
    CharArray,
    PString(bool /*flookup*/, u8 /*idx*/),
    VariantId,
}

/// The key definition as defined in the store.
/// The KeyDef is somehow independent of the other key :
/// It may be sementically dependent but this structure doesn't represent it.
/// The resolition of the depedencies is made later.
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct KeyDef {
    pub size: usize,
    pub kind: KeyDefKind,
}

impl Producable for KeyDef {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let keyinfo = stream.read_u8()?;
        let keytype = keyinfo >> 4;
        let keydata = keyinfo & 0x0F;
        let (keysize, kind) = match keytype {
            0b0000 => (keydata as u16 + 1, KeyDefKind::Padding),
            0b0001 => match keydata {
                0b0000 => (4, KeyDefKind::ContentAddress(false)),
                0b0001 => (4, KeyDefKind::ContentAddress(true)),
                _ => return Err(Error::FormatError),
            },
            0b0010 => (
                (keydata & 0x07) as u16 + 1,
                if (keydata & 0x08) != 0 {
                    KeyDefKind::SignedInt
                } else {
                    KeyDefKind::UnsignedInt
                },
            ),
            0b0100 => {
                (
                    if keydata & 0x08 == 0 {
                        (keydata + 1) as u16
                    } else {
                        // We need a complement byte
                        let complement = stream.read_u8()?;
                        (((keydata & 0x03) as u16) << 8) + complement as u16 + 9
                    },
                    KeyDefKind::CharArray,
                )
            }
            0b0110 | 0b0111 => {
                let flookup: bool = keytype & 0b1 != 0;
                let size = keydata as u16 + 1;
                let keystoreidx = stream.read_u8()?;
                (size, KeyDefKind::PString(flookup, keystoreidx))
            }
            0b1000 => (1, KeyDefKind::VariantId),
            _ => return Err(Error::FormatError),
        };
        Ok(Self {
            size: keysize as usize,
            kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(&[0b1000_0000] => KeyDef{size:1, kind:KeyDefKind::VariantId })]
    #[test_case(&[0b0000_0000] => KeyDef{size:1, kind:KeyDefKind::Padding })]
    #[test_case(&[0b0000_0111] => KeyDef{size:8, kind:KeyDefKind::Padding })]
    #[test_case(&[0b0001_0000] => KeyDef{size:4, kind:KeyDefKind::ContentAddress(false) })]
    #[test_case(&[0b0001_0001] => KeyDef{size:4, kind:KeyDefKind::ContentAddress(true) })]
    #[test_case(&[0b0010_0000] => KeyDef{size:1, kind:KeyDefKind::UnsignedInt })]
    #[test_case(&[0b0010_0010] => KeyDef{size:3, kind:KeyDefKind::UnsignedInt })]
    #[test_case(&[0b0010_0111] => KeyDef{size:8, kind:KeyDefKind::UnsignedInt })]
    #[test_case(&[0b0010_1000] => KeyDef{size:1, kind:KeyDefKind::SignedInt })]
    #[test_case(&[0b0010_1010] => KeyDef{size:3, kind:KeyDefKind::SignedInt })]
    #[test_case(&[0b0010_1111] => KeyDef{size:8, kind:KeyDefKind::SignedInt })]
    #[test_case(&[0b0100_0000] => KeyDef{size:1, kind:KeyDefKind::CharArray })]
    #[test_case(&[0b0100_0111] => KeyDef{size:8, kind:KeyDefKind::CharArray })]
    #[test_case(&[0b0100_1000, 0x00]=> KeyDef{size:9, kind:KeyDefKind::CharArray })]
    #[test_case(&[0b0100_1000, 0xFF]=> KeyDef{size:264, kind:KeyDefKind::CharArray })]
    #[test_case(&[0b0100_1011, 0xFF]=> KeyDef{size:1032, kind:KeyDefKind::CharArray })]
    #[test_case(&[0b0110_0000, 0x0F]=> KeyDef{size:1, kind:KeyDefKind::PString(false, 0x0F) })]
    #[test_case(&[0b0110_0111, 0x0F]=> KeyDef{size:8, kind:KeyDefKind::PString(false, 0x0F) })]
    #[test_case(&[0b0111_0000, 0x0F]=> KeyDef{size:1, kind:KeyDefKind::PString(true, 0x0F) })]
    #[test_case(&[0b0111_0111, 0x0F]=> KeyDef{size:8, kind:KeyDefKind::PString(true, 0x0F) })]
    fn test_keydef(source: &[u8]) -> KeyDef {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        KeyDef::produce(stream.as_mut()).unwrap()
    }
}
