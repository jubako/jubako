use crate::bases::types::*;
use crate::primitive::*;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct IOError {}

pub type Result<T> = std::result::Result<T, IOError>;

/// A buffer is a container of "raw data".
pub trait Buffer {
    fn read_data(&self, offset: Offset, end: ReadEnd) -> Result<&[u8]>;
    fn size(&self) -> Size;
}

impl Buffer for Vec<u8> {
    fn read_data(&self, offset: Offset, end: ReadEnd) -> Result<&[u8]> {
        assert!(offset.is_valid(self.size()));
        // We know offset < size < usize::MAX
        match end {
            End::None => {
                let offset = offset.0 as usize;
                Ok(&self[offset..])
            }
            End::Size(size) => {
                let end = offset + size;
                assert!(end.is_valid(self.size()));
                let offset = offset.0 as usize;
                let end = end.0 as usize;
                Ok(&self[offset..end])
            }
            End::Offset(end) => {
                assert!(end.is_valid(self.size()));
                let offset = offset.0 as usize;
                let end = end.0 as usize;
                Ok(&self[offset..end])
            }
        }
    }
    fn size(&self) -> Size {
        self.len().into()
    }
}

/// A producer is the main trait producing stuff from "raw data".
/// A producer may have a size, and is positionned.
/// The cursor can be move.
/// Producing a value "consumes" the data and the cursor is moved.
/// It is possible to create subproducer, a producer reading the sub range of tha data.
/// Each producer are independant.
/// Data is never modified.
pub trait Producer {
    fn read_data(&mut self, size: usize) -> Result<&[u8]>;
    fn move_cursor(&mut self, delta: Offset);
    fn set_cursor(&mut self, pos: Offset);
    fn teel_cursor(&self) -> Offset;
    fn size(&self) -> Size;

    /// Reset the cursor.
    /// Reseting the cursor do NOT set the cursor to position 0 (use `set_cursor`) for that.
    /// Reseting the cursor change the producer has if the origin of the pruducer is on the current
    /// cursor.
    fn reset(&mut self);

    fn sub_producer_at(&self, offset: Offset, end: ArxEnd) -> Box<dyn Producer>;

    fn read_u8(&mut self) -> Result<u8> {
        let v = read_u8(self.read_data(1)?);
        Ok(v)
    }
    fn read_u16(&mut self) -> Result<u16> {
        let v = read_u16(self.read_data(2)?);
        Ok(v)
    }
    fn read_u32(&mut self) -> Result<u32> {
        let v = read_u32(self.read_data(4)?);
        Ok(v)
    }
    fn read_u64(&mut self) -> Result<u64> {
        let v = read_u64(self.read_data(8)?);
        Ok(v)
    }
    fn read_sized(&mut self, size: usize) -> Result<u64> {
        let v = read_to_u64(size, self.read_data(size)?);
        Ok(v)
    }
    fn read_data_into<'a, 'b>(&'a mut self, size: usize, buf: &'b mut [u8]) -> Result<&'b [u8]> {
        buf.copy_from_slice(self.read_data(size)?);
        Ok(buf)
    }
}

pub struct BufferReader<T: Buffer> {
    buffer: Rc<T>,
    origin: Offset,
    end: Offset,
    offset: Offset,
}

impl<T: Buffer> BufferReader<T> {
    pub fn new(buffer: Rc<T>, origin: Offset, end: ArxEnd) -> Self {
        assert!(origin.is_valid(buffer.size()));
        match end {
            End::None => {
                let end = buffer.size().into();
                Self {
                    buffer,
                    origin,
                    end,
                    offset: 0.into(),
                }
            }
            End::Offset(o) => {
                assert!(o.is_valid(buffer.size()));
                Self {
                    buffer,
                    origin,
                    end: o,
                    offset: 0.into(),
                }
            }
            End::Size(s) => {
                let end = origin + s;
                assert!(end.is_valid(buffer.size()));
                Self {
                    buffer,
                    origin,
                    end,
                    offset: 0.into(),
                }
            }
        }
    }

    fn current_offset(&self) -> Offset {
        self.origin + self.offset
    }
}

impl<T: Buffer + 'static> Producer for BufferReader<T> {
    fn read_data(&mut self, size: usize) -> Result<&[u8]> {
        assert!((self.offset + size).is_valid(self.size()));
        let s = &self
            .buffer
            .read_data(self.current_offset(), End::Size(size))?;
        self.offset += size;
        Ok(s)
    }
    fn move_cursor(&mut self, delta: Offset) {
        self.offset += delta;
    }
    fn set_cursor(&mut self, offset: Offset) {
        self.offset = offset;
    }
    fn teel_cursor(&self) -> Offset {
        self.offset
    }
    fn size(&self) -> Size {
        self.end - self.origin
    }
    fn reset(&mut self) {
        self.origin += self.offset;
        self.offset = 0.into();
    }

    fn sub_producer_at(&self, offset: Offset, end: ArxEnd) -> Box<dyn Producer> {
        let origin = self.origin + offset;
        let end = match end {
            End::Offset(o) => End::Offset(self.origin + o),
            any => any,
        };
        Box::new(BufferReader::new(Rc::clone(&self.buffer), origin, end))
    }
}

#[cfg(test)]
mod tests {
    use super::{IOError, Parsable, Serializable};

    macro_rules! test_serial {
        ($what:expr, $size:expr, $expected:expr) => {{
            let mut buf: [u8; $size] = [0xFF; $size];
            assert_eq!($what.serial(&mut buf[..]), Ok($size));
            assert_eq!(buf, $expected);
        }};
    }

    #[test]
    fn serial_u8() {
        test_serial!(0_u8, 1, [0x00]);
        test_serial!(1_u8, 1, [0x01]);
        test_serial!(255_u8, 1, [0xff]);
        test_serial!(128_u8, 1, [0x80]);
    }

    #[test]
    fn serial_u16() {
        test_serial!(0_u16, 2, [0x00, 0x00]);
        test_serial!(1_u16, 2, [0x00, 0x01]);
        test_serial!(255_u16, 2, [0x00, 0xff]);
        test_serial!(128_u16, 2, [0x00, 0x80]);
        test_serial!(0x8000_u16, 2, [0x80, 0x00]);
        test_serial!(0xFF00_u16, 2, [0xFF, 0x00]);
        test_serial!(0xFFFF_u16, 2, [0xFF, 0xFF]);

        let mut buf: [u8; 1] = [0xFF; 1];
        assert_eq!(1_u16.serial(&mut buf[..]), Err(IOError {}));
    }

    #[test]
    fn serial_u32() {
        test_serial!(0_u32, 4, [0x00, 0x00, 0x00, 0x00]);
        test_serial!(1_u32, 4, [0x00, 0x00, 0x00, 0x01]);
        test_serial!(255_u32, 4, [0x00, 0x00, 0x00, 0xff]);
        test_serial!(128_u32, 4, [0x00, 0x00, 0x00, 0x80]);
        test_serial!(0x8000_u32, 4, [0x00, 0x00, 0x80, 0x00]);
        test_serial!(0xFF00_u32, 4, [0x00, 0x00, 0xFF, 0x00]);
        test_serial!(0xFFFF_u32, 4, [0x00, 0x00, 0xFF, 0xFF]);
        test_serial!(0xFF000000_u32, 4, [0xFF, 0x00, 0x00, 0x00]);
        test_serial!(0xFF0000_u32, 4, [0x00, 0xFF, 0x00, 0x00]);
        test_serial!(0xFFFFFFFF_u32, 4, [0xFF, 0xFF, 0xFF, 0xFF]);

        let mut buf: [u8; 2] = [0xFF; 2];
        assert_eq!(1_u32.serial(&mut buf[..]), Err(IOError {}));
    }

    #[test]
    fn serial_u64() {
        test_serial!(0_u64, 8, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        test_serial!(1_u64, 8, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
        test_serial!(
            0xFF_u64,
            8,
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF]
        );
        test_serial!(
            0xFF00_u64,
            8,
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00]
        );
        test_serial!(
            0xFF0000_u64,
            8,
            [0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00]
        );
        test_serial!(
            0xFF000000_u64,
            8,
            [0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00]
        );
        test_serial!(
            0xFF00000000_u64,
            8,
            [0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00]
        );
        test_serial!(
            0xFF0000000000_u64,
            8,
            [0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        test_serial!(
            0xFF000000000000_u64,
            8,
            [0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        test_serial!(
            0xFF00000000000000_u64,
            8,
            [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        test_serial!(
            0xFF00000000008000_u64,
            8,
            [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00]
        );
        test_serial!(
            0xFFFFFFFFFFFFFFFF_u64,
            8,
            [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
        );

        let mut buf: [u8; 2] = [0xFF; 2];
        assert_eq!(1_u64.serial(&mut buf[..]), Err(IOError {}));
    }

    macro_rules! test_parse {
        ($ty:ty, $buf:expr, $size:expr, $expected:expr) => {{
            let mut data: $ty = 0x88;
            assert_eq!(data.parse(&$buf), Ok($size));
            assert_eq!(data, $expected);
        }};
    }

    #[test]
    fn parse_u8() {
        test_parse!(u8, [0x00], 1, 0);
        test_parse!(u8, [0x01], 1, 1);
        test_parse!(u8, [0xff], 1, 255);
        test_parse!(u8, [0x80], 1, 128);
    }

    #[test]
    fn parse_u16() {
        test_parse!(u16, [0x00, 0x00], 2, 0);
        test_parse!(u16, [0x00, 0x01], 2, 1);
        test_parse!(u16, [0x00, 0xff], 2, 255);
        test_parse!(u16, [0x00, 0x80], 2, 128);
        test_parse!(u16, [0x80, 0x00], 2, 0x8000);
        test_parse!(u16, [0xFF, 0x00], 2, 0xFF00);
        test_parse!(u16, [0xFF, 0xFF], 2, 0xFFFF);
    }

    #[test]
    fn parse_u32() {
        test_parse!(u32, [0x00, 0x00, 0x00, 0x00], 4, 0);
        test_parse!(u32, [0x00, 0x00, 0x00, 0x01], 4, 1);
        test_parse!(u32, [0x00, 0x00, 0x00, 0xff], 4, 255);
        test_parse!(u32, [0x00, 0x00, 0x00, 0x80], 4, 128);
        test_parse!(u32, [0x00, 0x00, 0x80, 0x00], 4, 0x8000);
        test_parse!(u32, [0x00, 0x00, 0xFF, 0x00], 4, 0xFF00);
        test_parse!(u32, [0x00, 0x00, 0xFF, 0xFF], 4, 0xFFFF);
        test_parse!(u32, [0xFF, 0x00, 0x00, 0x00], 4, 0xFF000000);
        test_parse!(u32, [0x00, 0xFF, 0x00, 0x00], 4, 0xFF0000);
        test_parse!(u32, [0xFF, 0xFF, 0xFF, 0xFF], 4, 0xFFFFFFFF);
    }

    #[test]
    fn parse_u64() {
        test_parse!(u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 8, 0);
        test_parse!(u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01], 8, 1);
        test_parse!(
            u64,
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF],
            8,
            0xFF
        );
        test_parse!(
            u64,
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00],
            8,
            0xFF00
        );
        test_parse!(
            u64,
            [0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00],
            8,
            0xFF0000
        );
        test_parse!(
            u64,
            [0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00],
            8,
            0xFF000000
        );
        test_parse!(
            u64,
            [0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00],
            8,
            0xFF00000000
        );
        test_parse!(
            u64,
            [0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00],
            8,
            0xFF0000000000
        );
        test_parse!(
            u64,
            [0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            8,
            0xFF000000000000
        );
        test_parse!(
            u64,
            [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            8,
            0xFF00000000000000
        );
        test_parse!(
            u64,
            [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00],
            8,
            0xFF00000000008000
        );
        test_parse!(
            u64,
            [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            8,
            0xFFFFFFFFFFFFFFFF
        );
    }

}
