use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize))]
pub enum CompressionType {
    None = 0,
    Lz4 = 1,
    Lzma = 2,
    Zstd = 3,
}

impl Parsable for CompressionType {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let v = parser.read_u8()?;
        match v {
            0 => Ok(CompressionType::None),
            1 => Ok(CompressionType::Lz4),
            2 => Ok(CompressionType::Lzma),
            3 => Ok(CompressionType::Zstd),
            v => Err(format_error!(
                &format!("Invalid compression type ({v})"),
                parser
            )),
        }
    }
}

impl Serializable for CompressionType {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        ser.write_u8(*self as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressiontype() {
        let reader = CheckReader::from(vec![0x00, 0x01, 0x02, 0x03, 0x4, 0xFF]);
        let mut parser = reader.create_parser(Offset::zero(), 6.into()).unwrap();
        assert_eq!(
            CompressionType::parse(&mut parser).unwrap(),
            CompressionType::None
        );
        assert_eq!(
            CompressionType::parse(&mut parser).unwrap(),
            CompressionType::Lz4
        );
        assert_eq!(
            CompressionType::parse(&mut parser).unwrap(),
            CompressionType::Lzma
        );
        assert_eq!(
            CompressionType::parse(&mut parser).unwrap(),
            CompressionType::Zstd
        );
        assert_eq!(parser.tell(), Offset::new(4));
        assert!(CompressionType::parse(&mut parser).is_err());
    }
}
