mod buffer;
mod compression;
mod file;

use crate::bases::types::*;
use crate::bases::{ARegion, Range};
pub(crate) use compression::*;
pub use file::FileSource;
use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use super::BlockCheck;

pub(crate) trait Source: Sync + Send {
    fn size(&self) -> Size;
    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()>;
    fn read(&self, offset: Offset, buf: &mut [u8]) -> Result<usize>;
    fn get_slice(&self, region: ARegion, block_check: BlockCheck) -> Result<Cow<[u8]>>;

    fn into_memory_source(
        self: Arc<Self>,
        region: ARegion,
        block_check: BlockCheck,
    ) -> Result<(Arc<dyn MemorySource>, ARegion)>;

    fn display(&self) -> String;
}

pub(crate) trait MemorySource: Source {
    fn get_slice(&self, region: Range<usize>) -> Result<&[u8]>;

    /// Get a slice from the MemorySource
    ///
    /// # Safety
    ///
    /// `region` must point to a valid range in the memory source.
    unsafe fn get_slice_unchecked(&self, range: Range<usize>) -> Result<&[u8]>;
    fn into_source(self: Arc<Self>) -> Arc<dyn Source>;
}

impl fmt::Debug for dyn Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Source{{Size:{}}}", self.size()))
    }
}

impl fmt::Debug for dyn MemorySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("MemorySource{{Size:{}}}", self.size()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bases::{CheckReader, Parser};
    use std::io::{Cursor, Write};
    use tempfile::tempfile;
    use test_case::test_case;

    fn create_buf_reader(data: &[u8]) -> Option<CheckReader> {
        Some(data.to_vec().into())
    }

    fn create_file_reader(data: &[u8]) -> Option<CheckReader> {
        let mut file = tempfile().unwrap();
        file.write_all(data).unwrap();
        Some(FileSource::new(file).unwrap().into())
    }

    #[cfg(feature = "lz4")]
    fn create_lz4_reader(data: &[u8]) -> Option<CheckReader> {
        let compressed_content = {
            let compressed_content = Vec::new();
            let mut encoder = lz4::EncoderBuilder::new()
                .level(16)
                .build(Cursor::new(compressed_content))
                .unwrap();
            let mut incursor = Cursor::new(data);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            let (compressed_content, err) = encoder.finish();
            err.unwrap();
            compressed_content.into_inner()
        };
        let decoder = lz4::Decoder::new(Cursor::new(compressed_content)).unwrap();
        Some(SeekableDecoder::new(decoder, ASize::from(data.len())).into())
    }

    #[cfg(not(feature = "lz4"))]
    fn create_lz4_reader(_data: &[u8]) -> Option<CheckReader> {
        None
    }

    #[cfg(feature = "lzma")]
    fn create_lzma_reader(data: &[u8]) -> Option<CheckReader> {
        let compressed_content = {
            let compressed_content = Vec::new();
            let mut encoder = xz2::write::XzEncoder::new_stream(
                Cursor::new(compressed_content),
                xz2::stream::Stream::new_lzma_encoder(
                    &xz2::stream::LzmaOptions::new_preset(9).unwrap(),
                )
                .unwrap(),
            );
            let mut incursor = Cursor::new(data);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        let decoder = xz2::read::XzDecoder::new_stream(
            Cursor::new(compressed_content),
            xz2::stream::Stream::new_lzma_decoder(128 * 1024 * 1024).unwrap(),
        );
        Some(SeekableDecoder::new(decoder, ASize::from(data.len())).into())
    }

    #[cfg(not(feature = "lzma"))]
    fn create_lzma_reader(_data: &[u8]) -> Option<CheckReader> {
        None
    }

    #[cfg(feature = "zstd")]
    fn create_zstd_reader(data: &[u8]) -> Option<CheckReader> {
        let compressed_content = {
            let compressed_content = Vec::new();
            let mut encoder = zstd::Encoder::new(Cursor::new(compressed_content), 0).unwrap();
            let mut incursor = Cursor::new(data);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        let decoder = zstd::Decoder::new(Cursor::new(compressed_content)).unwrap();
        Some(SeekableDecoder::new(decoder, data.len().into()).into())
    }

    #[cfg(not(feature = "zstd"))]
    fn create_zstd_reader(_data: &[u8]) -> Option<CheckReader> {
        None
    }

    type ReaderCreator = fn(&[u8]) -> Option<CheckReader>;

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_parser(creator: ReaderCreator) {
        let reader = creator(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        if reader.is_none() {
            return;
        }
        let reader = reader.unwrap();
        let mut parser = reader.create_parser(Offset::zero(), 9.into()).unwrap();
        assert_eq!(parser.read_u8().unwrap(), 0x00_u8);
        assert_eq!(parser.tell(), Offset::new(1));
        assert_eq!(parser.read_u8().unwrap(), 0x01_u8);
        assert_eq!(parser.tell(), Offset::new(2));
        assert_eq!(parser.read_u16().unwrap(), 0x0302_u16);
        assert_eq!(parser.tell(), Offset::new(4));
        parser = reader.create_parser(Offset::zero(), 9.into()).unwrap();
        assert_eq!(parser.read_u32().unwrap(), 0x03020100_u32);
        assert_eq!(parser.read_u32().unwrap(), 0x07060504_u32);
        assert_eq!(parser.tell(), Offset::new(8));
        assert!(parser.read_u64().is_err());
        parser = reader.create_parser(Offset::zero(), 9.into()).unwrap();
        assert_eq!(parser.read_u64().unwrap(), 0x0706050403020100_u64);
        assert_eq!(parser.tell(), Offset::new(8));

        let mut parser1 = reader.create_parser(Offset::from(1_u64), 8.into()).unwrap();
        assert_eq!(parser1.tell(), Offset::zero());
        assert_eq!(parser1.read_u8().unwrap(), 0x01_u8);
        assert_eq!(parser1.tell(), Offset::new(1));
        assert_eq!(parser1.read_u16().unwrap(), 0x0302_u16);
        assert_eq!(parser1.tell(), Offset::new(3));
        assert_eq!(parser1.read_u32().unwrap(), 0x07060504_u32);
        assert_eq!(parser1.tell(), Offset::new(7));
        assert!(parser1.read_u64().is_err());
        parser1 = reader.create_parser(Offset::new(1), 8.into()).unwrap();
        assert_eq!(parser1.read_u64().unwrap(), 0x0807060504030201_u64);
        assert_eq!(parser1.tell(), Offset::new(8));

        parser = reader.create_parser(Offset::zero(), 9.into()).unwrap();
        parser1 = reader.create_parser(Offset::new(1), 8.into()).unwrap();
        parser.skip(1).unwrap();
        assert_eq!(parser.read_u8().unwrap(), parser1.read_u8().unwrap());
        assert_eq!(parser.read_u16().unwrap(), parser1.read_u16().unwrap());
        assert_eq!(parser.read_u32().unwrap(), parser1.read_u32().unwrap());
    }
}
