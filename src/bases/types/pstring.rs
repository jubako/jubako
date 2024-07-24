use crate::bases::*;

pub struct PString {}

impl PString {
    fn serialize_string_size(string: &[u8], max_len: u8, ser: &mut Serializer) -> IoResult<usize> {
        assert!(string.len() <= max_len.into());
        let mut written = 0;
        written += ser.write_u8(string.len() as u8)?;
        written += ser.write_data(string)?;
        Ok(written)
    }
    pub fn serialize_string(string: &[u8], ser: &mut Serializer) -> IoResult<usize> {
        Self::serialize_string_size(string, 255, ser)
    }

    pub fn serialize_string_padded(
        string: &[u8],
        size: u8,
        ser: &mut Serializer,
    ) -> IoResult<usize> {
        let mut written = 0;
        written += Self::serialize_string_size(string, size, ser)?;
        written += ser.write_data(vec![0; size as usize - string.len()].as_slice())?;
        Ok(written)
    }
}

impl Parsable for PString {
    type Output = Vec<u8>;
    fn parse(parser: &mut impl Parser) -> Result<Vec<u8>> {
        let size = parser.read_u8()?;
        Ok(parser.read_slice(size as usize)?.into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(&[0x00] => "")]
    #[test_case(&[0x01, 72] => "H")]
    #[test_case(&[0x02, 72, 101] => "He")]
    #[test_case(&[0x03, 72, 0xC3, 0xA9] => "Hé")]
    fn test_pstring(source: &[u8]) -> String {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let reader = Reader::from(content);
        String::from_utf8(
            reader
                .parse_in::<PString>(Offset::zero(), reader.size())
                .unwrap(),
        )
        .unwrap()
    }
}
