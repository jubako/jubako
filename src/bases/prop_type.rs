#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum PropType {
    Padding = 0b0000_0000,
    ContentAddress = 0b0001_0000,
    UnsignedInt = 0b0010_0000,
    SignedInt = 0b0011_0000,
    Array = 0b0101_0000,
    VariantId = 0b1000_0000,
    DeportedUnsignedInt = 0b1010_0000,
    DeportedSignedInt = 0b1011_0000,
}

impl TryFrom<u8> for PropType {
    type Error = String;
    fn try_from(v: u8) -> std::result::Result<Self, String> {
        match v {
            0b0000_0000 => Ok(Self::Padding),
            0b0001_0000 => Ok(Self::ContentAddress),
            0b0010_0000 => Ok(Self::UnsignedInt),
            0b0011_0000 => Ok(Self::SignedInt),
            0b0100_0000 => todo!(), // Redirection and SubRange
            0b0101_0000 => Ok(Self::Array),
            0b1000_0000 => Ok(Self::VariantId),
            0b1010_0000 => Ok(Self::DeportedUnsignedInt),
            0b1011_0000 => Ok(Self::DeportedSignedInt),
            _ => Err(format!("Invalid property type ({v})")),
        }
    }
}
