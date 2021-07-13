use crate::bases::*;
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::rc::Rc;

pub type FileReader = ReaderWrapper<RefCell<File>>;
pub type FileStream = StreamWrapper<RefCell<File>>;

impl FileReader {
    pub fn new(mut source: File, end: End) -> Self {
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
        }
    }

    fn read_exact(&self, offset: Offset, buf: &mut [u8]) -> Result<()> {
        let mut f = self.source.borrow_mut();
        f.seek(SeekFrom::Start((self.origin + offset).0))?;
        match f.read_exact(buf) {
            Err(e) => Err(e.into()),
            Ok(v) => Ok(v),
        }
    }
}

impl Reader for FileReader {
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
        let mut d = [0_u8; 1];
        self.read_exact(offset, &mut d)?;
        Ok(u8::from_be_bytes(d))
    }
    fn read_u16(&self, offset: Offset) -> Result<u16> {
        let mut d = [0_u8; 2];
        self.read_exact(offset, &mut d)?;
        Ok(u16::from_be_bytes(d))
    }
    fn read_u32(&self, offset: Offset) -> Result<u32> {
        let mut d = [0_u8; 4];
        self.read_exact(offset, &mut d)?;
        Ok(u32::from_be_bytes(d))
    }
    fn read_u64(&self, offset: Offset) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(offset, &mut d)?;
        Ok(u64::from_be_bytes(d))
    }
    fn read_sized(&self, offset: Offset, size: usize) -> Result<u64> {
        let mut d = [0_u8; 8];
        self.read_exact(offset, &mut d[8 - size..])?;
        Ok(u64::from_be_bytes(d))
    }
}

impl Read for FileStream {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempfile;

    #[test]
    fn test_file_stream() {
        let mut file = tempfile().unwrap();
        file.write_all(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
            .unwrap();
        let reader = FileReader::new(file, End::None);
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

        let mut stream1 = reader.create_stream(Offset(1), End::None);
        assert_eq!(stream1.tell(), Offset::from(0));
        assert_eq!(stream1.read_u8().unwrap(), 0x01_u8);
        assert_eq!(stream1.tell(), Offset::from(1));
        assert_eq!(stream1.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(stream1.tell(), Offset::from(3));
        assert_eq!(stream1.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(stream1.tell(), Offset::from(7));
        assert!(stream1.read_u64().is_err());
        stream1 = reader.create_stream(Offset(1), End::None);
        assert_eq!(stream1.read_u64().unwrap(), 0x0102030405060708_u64);
        assert_eq!(stream1.tell(), Offset::from(8));

        stream = reader.create_stream(Offset(0), End::None);
        stream1 = reader.create_stream(Offset(1), End::None);
        stream.skip(Size(1)).unwrap();
        assert_eq!(stream.read_u8().unwrap(), stream1.read_u8().unwrap());
        assert_eq!(stream.read_u16().unwrap(), stream1.read_u16().unwrap());
        assert_eq!(stream.read_u32().unwrap(), stream1.read_u32().unwrap());
    }
}
