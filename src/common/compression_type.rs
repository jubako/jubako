use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressionType {
    None = 0,
    Lz4 = 1,
    Lzma = 2,
    Zstd = 3,
}

impl Producable for CompressionType {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let v = stream.read_u8()?;
        match v {
            0 => Ok(CompressionType::None),
            1 => Ok(CompressionType::Lz4),
            2 => Ok(CompressionType::Lzma),
            3 => Ok(CompressionType::Zstd),
            v => Err(format_error!(
                &format!("Invalid compression type ({})", v),
                stream
            )),
        }
    }
}

impl Writable for CompressionType {
    fn write(&self, out_stream: &mut dyn OutStream) -> IoResult<()> {
        out_stream.write_u8(*self as u8)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressiontype() {
        let reader = BufReader::new(vec![0x00, 0x01, 0x02, 0x03, 0x4, 0xFF], End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            CompressionType::produce(stream.as_mut()).unwrap(),
            CompressionType::None
        );
        assert_eq!(
            CompressionType::produce(stream.as_mut()).unwrap(),
            CompressionType::Lz4
        );
        assert_eq!(
            CompressionType::produce(stream.as_mut()).unwrap(),
            CompressionType::Lzma
        );
        assert_eq!(
            CompressionType::produce(stream.as_mut()).unwrap(),
            CompressionType::Zstd
        );
        assert_eq!(stream.tell(), Offset::from(4));
        assert!(CompressionType::produce(stream.as_mut()).is_err());
    }
}
