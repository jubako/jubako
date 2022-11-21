use crate::bases::*;

pub struct PString {}

impl PString {
    fn write_string_size(
        string: &[u8],
        max_len: u8,
        stream: &mut dyn OutStream,
    ) -> IoResult<usize> {
        assert!(string.len() <= max_len.into());
        let mut written = 0;
        written += stream.write_u8(string.len() as u8)?;
        written += stream.write_data(string)?;
        Ok(written)
    }
    pub fn write_string(string: &[u8], stream: &mut dyn OutStream) -> IoResult<usize> {
        Self::write_string_size(string, 255, stream)
    }

    pub fn write_string_padded(
        string: &[u8],
        size: u8,
        stream: &mut dyn OutStream,
    ) -> IoResult<usize> {
        let mut written = 0;
        written += Self::write_string_size(string, size, stream)?;
        written += stream.write_data(vec![0; size as usize - string.len()].as_slice())?;
        Ok(written)
    }
}

impl Producable for PString {
    type Output = Vec<u8>;
    fn produce(stream: &mut Stream) -> Result<Vec<u8>> {
        let size = stream.read_u8()?;
        stream.read_vec(size as usize)
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
        let reader = Reader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        String::from_utf8(PString::produce(&mut stream).unwrap()).unwrap()
    }
}
