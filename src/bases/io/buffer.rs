use crate::bases::primitive::*;
use crate::bases::*;
use std::io::Read;
use std::rc::Rc;

pub type BufReader = ReaderWrapper<Vec<u8>>;

impl BufReader {
    pub fn new(source: Vec<u8>, end: End) -> Self {
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
        }
    }
}

impl Reader for BufReader {
    fn size(&self) -> Size {
        self.end - self.origin
    }

    fn create_stream(&self, offset: Offset, end: End) -> Box<dyn Stream> {
        let source = Rc::clone(&self.source);
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        assert!(end <= self.end);
        Box::new(StreamWrapper::new_from_parts(source, origin, end, origin))
    }

    fn create_sub_reader(&self, offset: Offset, end: End) -> Box<dyn Reader> {
        let source = Rc::clone(&self.source);
        let origin = self.origin + offset;
        let end = match end {
            End::None => self.end,
            End::Offset(o) => self.origin + o,
            End::Size(s) => origin + s,
        };
        assert!(end <= self.end);
        Box::new(ReaderWrapper {
            source,
            origin,
            end,
        })
    }

    fn read_u8(&self, offset: Offset) -> Result<u8> {
        let o = offset.0 as usize;
        Ok(read_u8(&self.source[o..o + 1]))
    }
    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let o = offset.0 as usize;
        Ok(read_u16(&self.source[o..o + 2]))
    }
    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let o = offset.0 as usize;
        Ok(read_u32(&self.source[o..o + 4]))
    }
    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let o = offset.0 as usize;
        Ok(read_u64(&self.source[o..o + 8]))
    }
    fn read_sized(&self, offset: Offset, size: usize) -> Result<u64> {
        let o = offset.0 as usize;
        Ok(read_to_u64(size, &self.source[o..o + size]))
    }
}

impl StreamWrapper<Vec<u8>> {
    fn slice(&self) -> &[u8] {
        let offset = self.offset.0 as usize;
        let end = self.end.0 as usize;
        &self.source[offset..end]
    }
}

impl Read for StreamWrapper<Vec<u8>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bases::reader::*;

    #[test]
    fn test_vec_stream() {
        let reader = BufReader::new(
            vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            End::None,
        );
        let mut stream = reader.create_stream(Offset(0), End::None);
        assert_eq!(stream.read_u8().unwrap(), 0x00_u8);
        assert_eq!(stream.tell(), Offset::from(1));
        assert_eq!(stream.read_u8().unwrap(), 0x01_u8);
        assert_eq!(stream.tell(), Offset::from(2));
        assert_eq!(stream.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(stream.tell(), Offset::from(4));
        stream = reader.create_stream(Offset(0), End::None);
        assert_eq!(stream.read_u32().unwrap(), 0x00010203_u32);
        assert_eq!(stream.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(stream.tell(), Offset::from(8));
        assert!(stream.read_u64().is_err());
        stream = reader.create_stream(Offset(0), End::None);
        assert_eq!(stream.read_u64().unwrap(), 0x0001020304050607_u64);
        assert_eq!(stream.tell(), Offset::from(8));

        let mut stream1 = reader.create_stream(1.into(), End::None);
        assert_eq!(stream1.tell(), Offset::from(0));
        assert_eq!(stream1.read_u8().unwrap(), 0x01_u8);
        assert_eq!(stream1.tell(), Offset::from(1));
        assert_eq!(stream1.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(stream1.tell(), Offset::from(3));
        assert_eq!(stream1.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(stream1.tell(), Offset::from(7));
        assert!(stream1.read_u64().is_err());
        stream1 = reader.create_stream(1.into(), End::None);
        assert_eq!(stream1.read_u64().unwrap(), 0x0102030405060708_u64);
        assert_eq!(stream1.tell(), Offset::from(8));

        stream = reader.create_stream(Offset(0), End::None);
        stream1 = reader.create_stream(1.into(), End::None);
        stream.skip(Size(1)).unwrap();
        assert_eq!(stream.read_u8().unwrap(), stream1.read_u8().unwrap());
        assert_eq!(stream.read_u16().unwrap(), stream1.read_u16().unwrap());
        assert_eq!(stream.read_u32().unwrap(), stream1.read_u32().unwrap());
    }
}
