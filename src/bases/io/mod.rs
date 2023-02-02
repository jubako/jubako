mod buffer;
mod compression;
mod file;

use crate::bases::types::*;
use crate::bases::Stream;
pub use buffer::*;
pub use compression::*;
pub use file::*;
use std::fmt;
use std::sync::Arc;

pub trait Source: Sync + Send {
    fn size(&self) -> Size;
    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()>;
    fn read(&self, offset: Offset, buf: &mut [u8]) -> Result<usize>;
    fn into_memory(
        self: Arc<Self>,
        offset: Offset,
        size: usize,
    ) -> Result<(Arc<dyn Source>, Offset, End)>;

    fn get_slice(&self, offset: Offset, end: Offset) -> Result<&[u8]>;

    fn slice_1(&self, offset: Offset) -> Result<[u8; 1]> {
        let mut buf = [0_u8; 1];
        self.read_exact(offset, &mut buf)?;
        Ok(buf)
    }

    fn slice_2(&self, offset: Offset) -> Result<[u8; 2]> {
        let mut buf = [0_u8; 2];
        self.read_exact(offset, &mut buf)?;
        Ok(buf)
    }

    fn slice_4(&self, offset: Offset) -> Result<[u8; 4]> {
        let mut buf = [0_u8; 4];
        self.read_exact(offset, &mut buf)?;
        Ok(buf)
    }

    fn slice_8(&self, offset: Offset) -> Result<[u8; 8]> {
        let mut buf = [0_u8; 8];
        self.read_exact(offset, &mut buf)?;
        Ok(buf)
    }

    fn slice_sized(&self, offset: Offset, size: ByteSize) -> Result<[u8; 8]> {
        let mut buf = [0_u8; 8];
        self.read_exact(offset, &mut buf[..size as usize])?;
        Ok(buf)
    }
}

impl fmt::Debug for dyn Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Source{{Size:{}}}", self.size()))
    }
}

/// A Producable is a object that can be produce from a stream.
pub trait Producable {
    type Output;
    fn produce(stream: &mut Stream) -> Result<Self::Output>
    where
        Self::Output: Sized;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bases::Reader;
    use std::io::{Cursor, Write};
    use tempfile::tempfile;
    use test_case::test_case;

    fn create_buf_reader(data: &[u8]) -> Reader {
        Reader::new(data.to_vec(), End::None)
    }

    fn create_file_reader(data: &[u8]) -> Reader {
        let mut file = tempfile().unwrap();
        file.write_all(data).unwrap();
        Reader::new(FileSource::new(file), End::None)
    }

    fn create_lz4_reader(data: &[u8]) -> Reader {
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
        Reader::new(
            Lz4Source::new(decoder, Size::from(data.len())),
            End::Size(Size::from(data.len())),
        )
    }

    fn create_lzma_reader(data: &[u8]) -> Reader {
        let compressed_content = {
            let compressed_content = Vec::new();
            let mut encoder =
                lzma::LzmaWriter::new_compressor(Cursor::new(compressed_content), 9).unwrap();
            let mut incursor = Cursor::new(data);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        let decoder = lzma::LzmaReader::new_decompressor(Cursor::new(compressed_content)).unwrap();
        Reader::new(
            LzmaSource::new(decoder, Size::from(data.len())),
            End::Size(Size::from(data.len())),
        )
    }

    fn create_zstd_reader(data: &[u8]) -> Reader {
        let compressed_content = {
            let compressed_content = Vec::new();
            let mut encoder = zstd::Encoder::new(Cursor::new(compressed_content), 0).unwrap();
            let mut incursor = Cursor::new(data);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        let decoder = zstd::Decoder::new(Cursor::new(compressed_content)).unwrap();
        Reader::new(
            ZstdSource::new(decoder, Size::from(data.len())),
            End::Size(Size::from(data.len())),
        )
    }

    type ReaderCreator = fn(&[u8]) -> Reader;

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_reader(creator: ReaderCreator) {
        let reader = creator(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(reader.read_u8(Offset::zero()).unwrap(), 0x00_u8);
        assert_eq!(reader.read_u8(Offset::new(1)).unwrap(), 0x01_u8);
        assert_eq!(reader.read_u16(Offset::new(2)).unwrap(), 0x0203_u16);
        assert_eq!(reader.read_u32(Offset::zero()).unwrap(), 0x00010203_u32);
        assert_eq!(reader.read_u32(Offset::new(4)).unwrap(), 0x04050607_u32);
        assert_eq!(
            reader.read_u64(Offset::zero()).unwrap(),
            0x0001020304050607_u64
        );

        assert_eq!(reader.read_i8(Offset::zero()).unwrap(), 0x00_i8);
        assert_eq!(reader.read_i8(Offset::new(1)).unwrap(), 0x01_i8);
        assert_eq!(reader.read_i16(Offset::new(2)).unwrap(), 0x0203_i16);
        assert_eq!(reader.read_i32(Offset::zero()).unwrap(), 0x00010203_i32);
        assert_eq!(reader.read_i32(Offset::new(4)).unwrap(), 0x04050607_i32);
        assert_eq!(
            reader.read_i64(Offset::zero()).unwrap(),
            0x0001020304050607_i64
        );

        assert_eq!(reader.read_u8(Offset::new(8)).unwrap(), 0x08_u8);
        assert!(reader.read_u8(Offset::new(9)).is_err());
        assert_eq!(reader.read_u16(Offset::new(7)).unwrap(), 0x0708_u16);
        assert!(reader.read_u16(Offset::new(8)).is_err());
        assert_eq!(reader.read_u32(Offset::new(5)).unwrap(), 0x05060708_u32);
        assert!(reader.read_u32(Offset::new(6)).is_err());
        assert_eq!(
            reader.read_u64(Offset::new(1)).unwrap(),
            0x0102030405060708_u64
        );
        assert!(reader.read_u64(Offset::new(2)).is_err());

        let reader1 = reader.create_sub_reader(Offset::new(1), End::None);
        assert_eq!(reader1.read_u8(Offset::zero()).unwrap(), 0x01_u8);
        assert_eq!(reader1.read_u16(Offset::new(1)).unwrap(), 0x0203_u16);
        assert_eq!(reader1.read_u32(Offset::new(3)).unwrap(), 0x04050607_u32);
        assert_eq!(
            reader1.read_u64(Offset::zero()).unwrap(),
            0x0102030405060708_u64
        );
        assert!(reader1.read_u64(Offset::new(1)).is_err());
    }

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_reader2(creator: ReaderCreator) {
        let reader = creator(&[0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xFF]);
        assert_eq!(reader.read_u8(Offset::zero()).unwrap(), 0xFE_u8);
        assert_eq!(reader.read_i8(Offset::zero()).unwrap(), -0x02_i8);
        assert_eq!(reader.read_u8(Offset::new(1)).unwrap(), 0xDC_u8);
        assert_eq!(reader.read_i8(Offset::new(1)).unwrap(), -0x24_i8);
        assert_eq!(reader.read_u16(Offset::zero()).unwrap(), 0xFEDC_u16);
        assert_eq!(reader.read_i16(Offset::zero()).unwrap(), -0x0124_i16);
        assert_eq!(reader.read_u16(Offset::new(2)).unwrap(), 0xBA98_u16);
        assert_eq!(reader.read_i16(Offset::new(2)).unwrap(), -0x4568_i16);
        assert_eq!(reader.read_u32(Offset::zero()).unwrap(), 0xFEDCBA98_u32);
        assert_eq!(reader.read_i32(Offset::zero()).unwrap(), -0x01234568_i32);
        assert_eq!(reader.read_u32(Offset::new(3)).unwrap(), 0x98765432_u32);
        assert_eq!(reader.read_i32(Offset::new(3)).unwrap(), -0x6789ABCE_i32);
        assert_eq!(reader.read_u32(Offset::new(4)).unwrap(), 0x76543210_u32);
        assert_eq!(reader.read_i32(Offset::new(4)).unwrap(), 0x76543210_i32);
        assert_eq!(
            reader.read_u64(Offset::zero()).unwrap(),
            0xFEDCBA9876543210_u64
        );
        assert_eq!(
            reader.read_i64(Offset::zero()).unwrap(),
            -0x0123456789ABCDF0_i64
        );
        assert_eq!(
            reader.read_u64(Offset::new(1)).unwrap(),
            0xDCBA9876543210FF_u64
        );
        assert_eq!(
            reader.read_i64(Offset::new(1)).unwrap(),
            -0x23456789ABCDEF01_i64
        );

        assert_eq!(
            reader.read_usized(Offset::zero(), ByteSize::U1).unwrap(),
            0xFE_u64
        );
        assert_eq!(
            reader.read_isized(Offset::zero(), ByteSize::U1).unwrap(),
            -0x02_i64
        );
        assert_eq!(
            reader.read_usized(Offset::new(1), ByteSize::U1).unwrap(),
            0xDC_u64
        );
        assert_eq!(
            reader.read_isized(Offset::new(1), ByteSize::U1).unwrap(),
            -0x24_i64
        );
        assert_eq!(
            reader.read_usized(Offset::zero(), ByteSize::U2).unwrap(),
            0xFEDC_u64
        );
        assert_eq!(
            reader.read_isized(Offset::zero(), ByteSize::U2).unwrap(),
            -0x0124_i64
        );
        assert_eq!(
            reader.read_usized(Offset::zero(), ByteSize::U3).unwrap(),
            0xFEDCBA_u64
        );
        assert_eq!(
            reader.read_isized(Offset::zero(), ByteSize::U3).unwrap(),
            -0x012346_i64
        );
        assert_eq!(
            reader.read_isized(Offset::new(1), ByteSize::U3).unwrap(),
            -0x234568_i64
        );
        assert_eq!(
            reader.read_usized(Offset::zero(), ByteSize::U4).unwrap(),
            0xFEDCBA98_u64
        );
        assert_eq!(
            reader.read_isized(Offset::zero(), ByteSize::U4).unwrap(),
            -0x01234568_i64
        );
        assert_eq!(
            reader.read_usized(Offset::zero(), ByteSize::U5).unwrap(),
            0xFEDCBA9876_u64
        );
        assert_eq!(
            reader.read_isized(Offset::zero(), ByteSize::U5).unwrap(),
            -0x012345678A_i64
        );
        assert_eq!(
            reader.read_usized(Offset::zero(), ByteSize::U6).unwrap(),
            0xFEDCBA987654_u64
        );
        assert_eq!(
            reader.read_isized(Offset::zero(), ByteSize::U6).unwrap(),
            -0x0123456789AC_i64
        );
        assert_eq!(
            reader.read_usized(Offset::zero(), ByteSize::U7).unwrap(),
            0xFEDCBA98765432_u64
        );
        assert_eq!(
            reader.read_isized(Offset::zero(), ByteSize::U7).unwrap(),
            -0x0123456789ABCE_i64
        );
        assert_eq!(
            reader.read_usized(Offset::zero(), ByteSize::U8).unwrap(),
            0xFEDCBA9876543210_u64
        );
        assert_eq!(
            reader.read_isized(Offset::zero(), ByteSize::U8).unwrap(),
            -0x0123456789ABCDF0_i64
        );
    }

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_stream(creator: ReaderCreator) {
        let reader = creator(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        let mut stream = reader.create_stream_all();
        assert_eq!(stream.read_u8().unwrap(), 0x00_u8);
        assert_eq!(stream.tell(), Offset::new(1));
        assert_eq!(stream.read_u8().unwrap(), 0x01_u8);
        assert_eq!(stream.tell(), Offset::new(2));
        assert_eq!(stream.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(stream.tell(), Offset::new(4));
        stream = reader.create_stream_all();
        assert_eq!(stream.read_u32().unwrap(), 0x00010203_u32);
        assert_eq!(stream.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(stream.tell(), Offset::new(8));
        assert!(stream.read_u64().is_err());
        stream = reader.create_stream_all();
        assert_eq!(stream.read_u64().unwrap(), 0x0001020304050607_u64);
        assert_eq!(stream.tell(), Offset::new(8));

        let mut stream1 = reader.create_stream_from(Offset::from(1_u64));
        assert_eq!(stream1.tell(), Offset::zero());
        assert_eq!(stream1.read_u8().unwrap(), 0x01_u8);
        assert_eq!(stream1.tell(), Offset::new(1));
        assert_eq!(stream1.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(stream1.tell(), Offset::new(3));
        assert_eq!(stream1.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(stream1.tell(), Offset::new(7));
        assert!(stream1.read_u64().is_err());
        stream1 = reader.create_stream_from(Offset::new(1));
        assert_eq!(stream1.read_u64().unwrap(), 0x0102030405060708_u64);
        assert_eq!(stream1.tell(), Offset::new(8));

        stream = reader.create_stream_from(Offset::zero());
        stream1 = reader.create_stream_from(Offset::new(1));
        stream.skip(Size::new(1)).unwrap();
        assert_eq!(stream.read_u8().unwrap(), stream1.read_u8().unwrap());
        assert_eq!(stream.read_u16().unwrap(), stream1.read_u16().unwrap());
        assert_eq!(stream.read_u32().unwrap(), stream1.read_u32().unwrap());
    }

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_create_sub_reader(creator: ReaderCreator) {
        let reader = creator(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(reader.size(), Size::new(9));
        let sub_reader = reader.create_sub_reader(Offset::zero(), End::None);
        assert_eq!(sub_reader.size(), Size::new(9));
        let sub_reader = reader.create_sub_reader(Offset::new(2), End::None);
        assert_eq!(sub_reader.size(), Size::new(7));
        let sub_reader = reader.create_sub_reader(Offset::zero(), End::new_size(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_sub_reader(Offset::new(2), End::new_size(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_sub_reader(Offset::zero(), End::new_offset(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_sub_reader(Offset::new(2), End::new_offset(6u64));
        assert_eq!(sub_reader.size(), Size::new(4));

        let reader = reader.create_sub_reader(Offset::new(1), End::None);
        assert_eq!(reader.size(), Size::new(8));
        let sub_reader = reader.create_sub_reader(Offset::zero(), End::None);
        assert_eq!(sub_reader.size(), Size::new(8));
        let sub_reader = reader.create_sub_reader(Offset::new(2), End::None);
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_sub_reader(Offset::zero(), End::new_size(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_sub_reader(Offset::new(2), End::new_size(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_sub_reader(Offset::zero(), End::new_offset(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_sub_reader(Offset::new(2), End::new_offset(6u64));
        assert_eq!(sub_reader.size(), Size::new(4));
    }

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_create_sub_stream(creator: ReaderCreator) {
        let reader = creator(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(reader.size(), Size::new(9));
        let sub_reader = reader.create_stream(Offset::zero(), End::None);
        assert_eq!(sub_reader.size(), Size::new(9));
        let sub_reader = reader.create_stream(Offset::new(2), End::None);
        assert_eq!(sub_reader.size(), Size::new(7));
        let sub_reader = reader.create_stream(Offset::zero(), End::new_size(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_stream(Offset::new(2), End::new_size(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_stream(Offset::zero(), End::new_offset(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_stream(Offset::new(2), End::new_offset(6u64));
        assert_eq!(sub_reader.size(), Size::new(4));

        let reader = reader.create_sub_reader(Offset::new(1), End::None);
        assert_eq!(reader.size(), Size::new(8));
        let sub_reader = reader.create_stream(Offset::zero(), End::None);
        assert_eq!(sub_reader.size(), Size::new(8));
        let sub_reader = reader.create_stream(Offset::new(2), End::None);
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_stream(Offset::zero(), End::new_size(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_stream(Offset::new(2), End::new_size(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_stream(Offset::zero(), End::new_offset(6u64));
        assert_eq!(sub_reader.size(), Size::new(6));
        let sub_reader = reader.create_stream(Offset::new(2), End::new_offset(6u64));
        assert_eq!(sub_reader.size(), Size::new(4));
    }
}
