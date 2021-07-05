use crate::bases::types::*;
use crate::bases::producing::*;
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use std::io::{Read, Seek, SeekFrom, ErrorKind};

pub struct ProducerWrapper<T> {
    source: Rc<T>,
    origin: Offset,
    end: Offset,
    offset: Offset,
}

impl ProducerWrapper<Vec<u8>> {
    pub fn new(source: Vec<u8>, end: ArxEnd) -> Self {
        let source = Rc::new(source);
        let end = match end {
            End::None => Offset(source.len() as u64),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        assert!(end.is_valid(source.len().into()));
        Self {
            source,
            end,
            origin: Offset(0),
            offset: Offset(0),
        }
    }
    fn slice(&self) -> &[u8] {
        let origin = self.origin.0 as usize;
        let end = self.end.0 as usize;
        &self.source[origin..end]
    }
}

impl ProducerWrapper<RefCell<File>> {
    pub fn new(mut source: File, end: ArxEnd) -> Self {
        let len = source.seek(SeekFrom::End(0)).unwrap();
        let source = Rc::new(RefCell::new(source));
        let end = match end {
            End::None => Offset(len as u64),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        assert!(end.is_valid(len.into()));
        Self {
            source,
            end,
            origin: Offset(0),
            offset: Offset(0),
        }
    }
}

impl<T> Seek for ProducerWrapper<T> {
    fn seek(&mut self, pos: SeekFrom) -> std::result::Result<u64, std::io::Error> {
        let new: Offset = match pos {
            SeekFrom::Start(pos) => self.origin + Offset::from(pos),
            SeekFrom::End(delta) => {
                if delta.is_positive() {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "It is not possible to seek after the end.",
                    ));
                }
                Offset::from(self.end.0 - delta.abs() as u64)
            }
            SeekFrom::Current(delta) => {
                if delta.is_positive() {
                    self.offset + Offset::from(delta as u64)
                } else {
                    (self.offset - Offset::from(delta.abs() as u64)).into()
                }
            }
        };
        if new < self.origin || new > self.end {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "Final position is not valid",
            ));
        }
        self.offset = new;
        Ok((self.offset - self.origin).0)
    }
}

impl Read for ProducerWrapper<Vec<u8>> {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let mut slice = self.slice();
        match slice.read(buf) {
            Ok(s) => {
                self.offset += s;
                Ok(s)
            }
            err => err,
        }
    }
}

impl Read for ProducerWrapper<RefCell<File>> {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let mut file = self.source.as_ref().borrow_mut();
        file.seek(SeekFrom::Start(self.offset.0))?;
        match file.read(buf) {
            Ok(s) => {
                self.offset += s;
                Ok(s)
            }
            err => err,
        }
    }
}

impl<T: 'static> Producer for ProducerWrapper<T>
where
    ProducerWrapper<T>: std::io::Read,
{
    fn teel_cursor(&self) -> Offset {
        (self.offset - self.origin).into()
    }
    fn size(&self) -> Size {
        self.end - self.origin
    }

    fn sub_producer_at(&self, offset: Offset, end: ArxEnd) -> Box<dyn Producer> {
        let origin = self.origin + offset;
        assert!(origin <= self.end);
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        assert!(end <= self.end);
        Box::new(ProducerWrapper::<T> {
            source: Rc::clone(&self.source),
            origin,
            end,
            offset: origin,
        })
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
