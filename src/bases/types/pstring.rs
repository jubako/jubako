use crate::bases::*;
use std::{
    borrow::{Borrow, Cow},
    marker::PhantomData,
};

pub struct PArray<Output> {
    _output: PhantomData<Output>,
}

impl<Output> PArray<Output> {
    fn serialize_string_size<T>(string: &T, max_len: u8, ser: &mut Serializer) -> IoResult<usize>
    where
        Output: Borrow<T>,
        T: AsRef<[u8]> + ?Sized,
    {
        let string = string.as_ref();
        assert!(string.len() <= max_len.into());
        let mut written = 0;
        written += ser.write_u8(string.len() as u8)?;
        written += ser.write_data(string)?;
        Ok(written)
    }
    pub(crate) fn serialize_string<T>(string: &T, ser: &mut Serializer) -> IoResult<usize>
    where
        Output: Borrow<T>,
        T: AsRef<[u8]> + ?Sized,
    {
        Self::serialize_string_size(string, 255, ser)
    }

    pub(crate) fn serialize_string_padded<T>(
        string: &T,
        size: u8,
        ser: &mut Serializer,
    ) -> IoResult<usize>
    where
        Output: Borrow<T>,
        T: AsRef<[u8]> + ?Sized,
    {
        let mut written = 0;
        written += Self::serialize_string_size(string, size, ser)?;
        written += ser.write_data(vec![0; size as usize - string.as_ref().len()].as_slice())?;
        Ok(written)
    }
}

impl Parsable for PArray<SmallBytes> {
    type Output = SmallBytes;
    fn parse(parser: &mut impl Parser) -> Result<SmallBytes> {
        let size = parser.read_u8()?;
        match parser.read_slice(size as usize)? {
            Cow::Borrowed(slice) => Ok(slice.into()),
            Cow::Owned(vec) => Ok(vec.into()),
        }
    }
}

impl Parsable for PArray<String> {
    type Output = SmallString;
    fn parse(parser: &mut impl Parser) -> Result<SmallString> {
        let data = PArray::<SmallBytes>::parse(parser)?;
        Ok(SmallString::from_byte_vec(data)?)
    }
}

pub type PBytes = PArray<SmallBytes>;
pub type PString = PArray<String>;

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
        let reader = CheckReader::from(content);
        reader
            .parse_in::<PString>(Offset::zero(), reader.size().try_into().unwrap())
            .unwrap()
            .to_string()
    }

    #[test_case(&[0x00] => b"".as_slice())]
    #[test_case(&[0x01, 72] => b"H".as_slice())]
    #[test_case(&[0x02, 72, 101] => b"He".as_slice())]
    #[test_case(&[0x03, 72, 0xC3, 0xA9] => "Hé".as_bytes())]
    fn test_pvec(source: &[u8]) -> Vec<u8> {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let reader = CheckReader::from(content);
        reader
            .parse_in::<PBytes>(Offset::zero(), reader.size().try_into().unwrap())
            .unwrap()
            .to_vec()
    }
}
