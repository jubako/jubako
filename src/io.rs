use crate::bases::types::*;
use crate::bases::producing::*;
use std::rc::Rc;
use std::io::{Read, Seek, SeekFrom, ErrorKind};
use std::cmp::min;

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
                    offset: origin,
                }
            }
            End::Offset(o) => {
                assert!(o.is_valid(buffer.size()));
                Self {
                    buffer,
                    origin,
                    end: o,
                    offset: origin,
                }
            }
            End::Size(s) => {
                let end = origin + s;
                assert!(end.is_valid(buffer.size()));
                Self {
                    buffer,
                    origin,
                    end,
                    offset: origin
                }
            }
        }
    }
}

impl<T: Buffer> Seek for BufferReader<T> {
    fn seek(&mut self, pos: SeekFrom) -> std::result::Result<u64, std::io::Error> {
        let new : Offset = match pos {
            SeekFrom::Start(pos) => {
                self.origin + Offset::from(pos)
            },
            SeekFrom::End(delta) => {
                if delta.is_positive() {
                    return Err(std::io::Error::new(ErrorKind::InvalidInput, "It is not possible to seek after the end."))
                }
                Offset::from(self.end.0 - delta.abs() as u64)
            },
            SeekFrom::Current(delta) => {
                if delta.is_positive() {
                    self.offset + Offset::from(delta as u64)
                } else {
                    (self.offset - Offset::from(delta.abs() as u64)).into()
                }
            }
        };
        if new < self.origin || new > self.end {
            return Err(std::io::Error::new(ErrorKind::Other, "Final position is not valid"))
        }
        self.offset = new;
        Ok((self.offset-self.origin).0)
    }
/*
    fn stream_len(&mut self) -> Result<u64> {
        Ok(self.end - self.origin)
    }

    fn stream_position(&mut self) -> Result<u64> {
        Ok(self.offset - self.origin)
    }
*/
}

impl<T: Buffer> Read for BufferReader<T> {
    fn read(&mut self, buf: &mut[u8]) -> std::result::Result<usize, std::io::Error> {
        let to_read = min(buf.len() as u64, self.end.0-self.offset.0) as usize;
        let s = &self.buffer.read_data(self.offset, End::Size(to_read)).unwrap();
        buf.copy_from_slice(&s);
        self.offset += to_read;
        Ok(to_read)
    }
}

impl<T: Buffer + 'static> Producer for BufferReader<T> {
    fn teel_cursor(&self) -> Offset {
        (self.offset - self.origin).into()
    }
    fn size(&self) -> Size {
        self.end - self.origin
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
