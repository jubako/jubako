mod buffer;
mod compression;
mod file;

use crate::bases::stream::*;
use crate::bases::types::*;
pub use buffer::*;
pub use compression::*;
pub use file::*;
use std::rc::Rc;

// A wrapper arount someting to implement Reader trait
pub struct ReaderWrapper<T> {
    source: Rc<T>,
    origin: Offset,
    end: Offset,
}

// A wrapper arount someting to implement Stream trait
pub struct StreamWrapper<T> {
    source: Rc<T>,
    origin: Offset,
    end: Offset,
    offset: Offset,
}

impl<T> StreamWrapper<T> {
    pub fn new_from_parts(source: Rc<T>, origin: Offset, end: Offset, offset: Offset) -> Self {
        Self {
            source,
            origin,
            end,
            offset,
        }
    }
}

impl<T: 'static> Stream for StreamWrapper<T>
where
    StreamWrapper<T>: std::io::Read,
{
    fn tell(&self) -> Offset {
        (self.offset - self.origin).into()
    }
    fn size(&self) -> Size {
        self.end - self.origin
    }
    fn skip(&mut self, size: Size) -> Result<()> {
        let new_offset = self.offset + size;
        if new_offset <= self.end {
            self.offset = new_offset;
            Ok(())
        } else {
            Err(Error::FormatError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bases::Reader;
    use std::io::{Cursor, Write};
    use tempfile::tempfile;
    use test_case::test_case;

    fn create_buf_reader(data: &[u8]) -> Box<dyn Reader> {
        Box::new(BufReader::new(data.to_vec(), End::None))
    }

    fn create_file_reader(data: &[u8]) -> Box<dyn Reader> {
        let mut file = tempfile().unwrap();
        file.write_all(data).unwrap();
        Box::new(FileReader::new(file, End::None))
    }

    fn create_lz4_reader(data: &[u8]) -> Box<dyn Reader> {
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
        Box::new(Lz4Reader::new(decoder, Size::from(data.len())))
    }

    fn create_lzma_reader(data: &[u8]) -> Box<dyn Reader> {
        let compressed_content = {
            let compressed_content = Vec::new();
            let mut encoder =
                lzma::LzmaWriter::new_compressor(Cursor::new(compressed_content), 9).unwrap();
            let mut incursor = Cursor::new(data);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        let decoder = lzma::LzmaReader::new_decompressor(Cursor::new(compressed_content)).unwrap();
        Box::new(LzmaReader::new(decoder, Size::from(data.len())))
    }

    fn create_zstd_reader(data: &[u8]) -> Box<dyn Reader> {
        let compressed_content = {
            let compressed_content = Vec::new();
            let mut encoder = zstd::Encoder::new(Cursor::new(compressed_content), 0).unwrap();
            let mut incursor = Cursor::new(data);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        let decoder = zstd::Decoder::new(Cursor::new(compressed_content)).unwrap();
        Box::new(ZstdReader::new(decoder, Size::from(data.len())))
    }

    type ReaderCreator = fn(&[u8]) -> Box<dyn Reader>;

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_reader(creator: ReaderCreator) {
        let reader = creator(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(reader.read_u8(Offset(0)).unwrap(), 0x00_u8);
        assert_eq!(reader.read_u8(Offset(1)).unwrap(), 0x01_u8);
        assert_eq!(reader.read_u16(Offset(2)).unwrap(), 0x0203_u16);
        assert_eq!(reader.read_u32(Offset(0)).unwrap(), 0x00010203_u32);
        assert_eq!(reader.read_u32(Offset(4)).unwrap(), 0x04050607_u32);
        assert_eq!(reader.read_u64(Offset(0)).unwrap(), 0x0001020304050607_u64);

        assert_eq!(reader.read_i8(Offset(0)).unwrap(), 0x00_i8);
        assert_eq!(reader.read_i8(Offset(1)).unwrap(), 0x01_i8);
        assert_eq!(reader.read_i16(Offset(2)).unwrap(), 0x0203_i16);
        assert_eq!(reader.read_i32(Offset(0)).unwrap(), 0x00010203_i32);
        assert_eq!(reader.read_i32(Offset(4)).unwrap(), 0x04050607_i32);
        assert_eq!(reader.read_i64(Offset(0)).unwrap(), 0x0001020304050607_i64);

        assert_eq!(reader.read_u8(Offset(8)).unwrap(), 0x08_u8);
        assert!(reader.read_u8(Offset(9)).is_err());
        assert_eq!(reader.read_u16(Offset(7)).unwrap(), 0x0708_u16);
        assert!(reader.read_u16(Offset(8)).is_err());
        assert_eq!(reader.read_u32(Offset(5)).unwrap(), 0x05060708_u32);
        assert!(reader.read_u32(Offset(6)).is_err());
        assert_eq!(reader.read_u64(Offset(1)).unwrap(), 0x0102030405060708_u64);
        assert!(reader.read_u64(Offset(2)).is_err());

        let reader1 = reader.create_sub_reader(1.into(), End::None);
        assert_eq!(reader1.read_u8(Offset(0)).unwrap(), 0x01_u8);
        assert_eq!(reader1.read_u16(Offset(1)).unwrap(), 0x0203_u16);
        assert_eq!(reader1.read_u32(Offset(3)).unwrap(), 0x04050607_u32);
        assert_eq!(reader1.read_u64(Offset(0)).unwrap(), 0x0102030405060708_u64);
        assert!(reader1.read_u64(Offset(1)).is_err());
    }

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_reader2(creator: ReaderCreator) {
        let reader = creator(&[0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xFF]);
        assert_eq!(reader.read_u8(Offset(0)).unwrap(), 0xFE_u8);
        assert_eq!(reader.read_i8(Offset(0)).unwrap(), -0x02_i8);
        assert_eq!(reader.read_u8(Offset(1)).unwrap(), 0xDC_u8);
        assert_eq!(reader.read_i8(Offset(1)).unwrap(), -0x24_i8);
        assert_eq!(reader.read_u16(Offset(0)).unwrap(), 0xFEDC_u16);
        assert_eq!(reader.read_i16(Offset(0)).unwrap(), -0x0124_i16);
        assert_eq!(reader.read_u16(Offset(2)).unwrap(), 0xBA98_u16);
        assert_eq!(reader.read_i16(Offset(2)).unwrap(), -0x4568_i16);
        assert_eq!(reader.read_u32(Offset(0)).unwrap(), 0xFEDCBA98_u32);
        assert_eq!(reader.read_i32(Offset(0)).unwrap(), -0x01234568_i32);
        assert_eq!(reader.read_u32(Offset(3)).unwrap(), 0x98765432_u32);
        assert_eq!(reader.read_i32(Offset(3)).unwrap(), -0x6789ABCE_i32);
        assert_eq!(reader.read_u32(Offset(4)).unwrap(), 0x76543210_u32);
        assert_eq!(reader.read_i32(Offset(4)).unwrap(), 0x76543210_i32);
        assert_eq!(reader.read_u64(Offset(0)).unwrap(), 0xFEDCBA9876543210_u64);
        assert_eq!(reader.read_i64(Offset(0)).unwrap(), -0x0123456789ABCDF0_i64);
        assert_eq!(reader.read_u64(Offset(1)).unwrap(), 0xDCBA9876543210FF_u64);
        assert_eq!(reader.read_i64(Offset(1)).unwrap(), -0x23456789ABCDEF01_i64);

        assert_eq!(reader.read_usized(Offset(0), 1).unwrap(), 0xFE_u64);
        assert_eq!(reader.read_isized(Offset(0), 1).unwrap(), -0x02_i64);
        assert_eq!(reader.read_usized(Offset(1), 1).unwrap(), 0xDC_u64);
        assert_eq!(reader.read_isized(Offset(1), 1).unwrap(), -0x24_i64);
        assert_eq!(reader.read_usized(Offset(0), 2).unwrap(), 0xFEDC_u64);
        assert_eq!(reader.read_isized(Offset(0), 2).unwrap(), -0x0124_i64);
        assert_eq!(reader.read_usized(Offset(0), 3).unwrap(), 0xFEDCBA_u64);
        assert_eq!(reader.read_isized(Offset(0), 3).unwrap(), -0x012346_i64);
        assert_eq!(reader.read_isized(Offset(1), 3).unwrap(), -0x234568_i64);
        assert_eq!(reader.read_usized(Offset(0), 4).unwrap(), 0xFEDCBA98_u64);
        assert_eq!(reader.read_isized(Offset(0), 4).unwrap(), -0x01234568_i64);
        assert_eq!(reader.read_usized(Offset(0), 5).unwrap(), 0xFEDCBA9876_u64);
        assert_eq!(reader.read_isized(Offset(0), 5).unwrap(), -0x012345678A_i64);
        assert_eq!(
            reader.read_usized(Offset(0), 6).unwrap(),
            0xFEDCBA987654_u64
        );
        assert_eq!(
            reader.read_isized(Offset(0), 6).unwrap(),
            -0x0123456789AC_i64
        );
        assert_eq!(
            reader.read_usized(Offset(0), 7).unwrap(),
            0xFEDCBA98765432_u64
        );
        assert_eq!(
            reader.read_isized(Offset(0), 7).unwrap(),
            -0x0123456789ABCE_i64
        );
        assert_eq!(
            reader.read_usized(Offset(0), 8).unwrap(),
            0xFEDCBA9876543210_u64
        );
        assert_eq!(
            reader.read_isized(Offset(0), 8).unwrap(),
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
        assert_eq!(stream.tell(), Offset::from(1));
        assert_eq!(stream.read_u8().unwrap(), 0x01_u8);
        assert_eq!(stream.tell(), Offset::from(2));
        assert_eq!(stream.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(stream.tell(), Offset::from(4));
        stream = reader.create_stream_all();
        assert_eq!(stream.read_u32().unwrap(), 0x00010203_u32);
        assert_eq!(stream.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(stream.tell(), Offset::from(8));
        assert!(stream.read_u64().is_err());
        stream = reader.create_stream_all();
        assert_eq!(stream.read_u64().unwrap(), 0x0001020304050607_u64);
        assert_eq!(stream.tell(), Offset::from(8));

        let mut stream1 = reader.create_stream_from(Offset(1));
        assert_eq!(stream1.tell(), Offset::from(0));
        assert_eq!(stream1.read_u8().unwrap(), 0x01_u8);
        assert_eq!(stream1.tell(), Offset::from(1));
        assert_eq!(stream1.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(stream1.tell(), Offset::from(3));
        assert_eq!(stream1.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(stream1.tell(), Offset::from(7));
        assert!(stream1.read_u64().is_err());
        stream1 = reader.create_stream_from(Offset(1));
        assert_eq!(stream1.read_u64().unwrap(), 0x0102030405060708_u64);
        assert_eq!(stream1.tell(), Offset::from(8));

        stream = reader.create_stream_from(Offset(0));
        stream1 = reader.create_stream_from(Offset(1));
        stream.skip(Size(1)).unwrap();
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
        assert_eq!(reader.size().0, 9);
        let sub_reader = reader.create_sub_reader(Offset(0), End::None);
        assert_eq!(sub_reader.size().0, 9);
        let sub_reader = reader.create_sub_reader(Offset(2), End::None);
        assert_eq!(sub_reader.size().0, 7);
        let sub_reader = reader.create_sub_reader(Offset(0), End::Size(Size(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_sub_reader(Offset(2), End::Size(Size(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_sub_reader(Offset(0), End::Offset(Offset(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_sub_reader(Offset(2), End::Offset(Offset(6)));
        assert_eq!(sub_reader.size().0, 4);

        let reader = reader.create_sub_reader(Offset(1), End::None);
        assert_eq!(reader.size().0, 8);
        let sub_reader = reader.create_sub_reader(Offset(0), End::None);
        assert_eq!(sub_reader.size().0, 8);
        let sub_reader = reader.create_sub_reader(Offset(2), End::None);
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_sub_reader(Offset(0), End::Size(Size(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_sub_reader(Offset(2), End::Size(Size(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_sub_reader(Offset(0), End::Offset(Offset(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_sub_reader(Offset(2), End::Offset(Offset(6)));
        assert_eq!(sub_reader.size().0, 4);
    }

    #[test_case(create_buf_reader)]
    #[test_case(create_file_reader)]
    #[test_case(create_lz4_reader)]
    #[test_case(create_lzma_reader)]
    #[test_case(create_zstd_reader)]
    fn test_create_sub_stream(creator: ReaderCreator) {
        let reader = creator(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(reader.size().0, 9);
        let sub_reader = reader.create_stream(Offset(0), End::None);
        assert_eq!(sub_reader.size().0, 9);
        let sub_reader = reader.create_stream(Offset(2), End::None);
        assert_eq!(sub_reader.size().0, 7);
        let sub_reader = reader.create_stream(Offset(0), End::Size(Size(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_stream(Offset(2), End::Size(Size(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_stream(Offset(0), End::Offset(Offset(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_stream(Offset(2), End::Offset(Offset(6)));
        assert_eq!(sub_reader.size().0, 4);

        let reader = reader.create_sub_reader(Offset(1), End::None);
        assert_eq!(reader.size().0, 8);
        let sub_reader = reader.create_stream(Offset(0), End::None);
        assert_eq!(sub_reader.size().0, 8);
        let sub_reader = reader.create_stream(Offset(2), End::None);
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_stream(Offset(0), End::Size(Size(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_stream(Offset(2), End::Size(Size(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_stream(Offset(0), End::Offset(Offset(6)));
        assert_eq!(sub_reader.size().0, 6);
        let sub_reader = reader.create_stream(Offset(2), End::Offset(Offset(6)));
        assert_eq!(sub_reader.size().0, 4);
    }
}
