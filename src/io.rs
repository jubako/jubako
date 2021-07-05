use crate::bases::producing::*;
use crate::bases::types::*;
use std::cell::RefCell;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::rc::Rc;

pub struct ProducerWrapper<T> {
    source: Rc<T>,
    origin: Offset,
    end: Offset,
    offset: Offset,
}

impl ProducerWrapper<Vec<u8>> {
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
            offset: Offset(0),
        }
    }
    fn slice(&self) -> &[u8] {
        let offset = self.offset.0 as usize;
        let end = self.end.0 as usize;
        &self.source[offset..end]
    }
}

impl ProducerWrapper<RefCell<File>> {
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
    fn tell_cursor(&self) -> Offset {
        (self.offset - self.origin).into()
    }
    fn size(&self) -> Size {
        self.end - self.origin
    }

    fn sub_producer_at(&self, offset: Offset, end: End) -> Box<dyn Producer> {
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
    use super::*;
    use std::io::Write;
    use tempfile::tempfile;

    #[test]
    fn test_vec_producer() {
        let mut producer = ProducerWrapper::<Vec<u8>>::new(
            vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            End::None,
        );
        assert_eq!(producer.read_u8().unwrap(), 0x00_u8);
        assert_eq!(producer.tell_cursor(), Offset::from(1));
        assert_eq!(producer.read_u8().unwrap(), 0x01_u8);
        assert_eq!(producer.tell_cursor(), Offset::from(2));
        assert_eq!(producer.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(producer.tell_cursor(), Offset::from(4));
        producer.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(producer.read_u32().unwrap(), 0x00010203_u32);
        assert_eq!(producer.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(producer.tell_cursor(), Offset::from(8));
        assert!(producer.read_u64().is_err());
        producer.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(producer.read_u64().unwrap(), 0x0001020304050607_u64);
        assert_eq!(producer.tell_cursor(), Offset::from(8));

        let mut sub_producer = producer.sub_producer_at(1.into(), End::None);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(0));
        assert_eq!(sub_producer.read_u8().unwrap(), 0x01_u8);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(1));
        assert_eq!(sub_producer.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(3));
        assert_eq!(sub_producer.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(7));
        assert!(sub_producer.read_u64().is_err());
        sub_producer.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(sub_producer.read_u64().unwrap(), 0x0102030405060708_u64);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(8));

        producer.seek(SeekFrom::Start(1)).unwrap();
        sub_producer.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(producer.read_u8().unwrap(), sub_producer.read_u8().unwrap());
        assert_eq!(
            producer.read_u16().unwrap(),
            sub_producer.read_u16().unwrap()
        );
        assert_eq!(
            producer.read_u32().unwrap(),
            sub_producer.read_u32().unwrap()
        );
    }

    #[test]
    fn test_file_producer() {
        let mut file = tempfile().unwrap();
        file.write_all(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
            .unwrap();
        let mut producer = ProducerWrapper::<RefCell<File>>::new(file, End::None);
        assert_eq!(producer.read_u8().unwrap(), 0x00_u8);
        assert_eq!(producer.tell_cursor(), Offset::from(1));
        assert_eq!(producer.read_u8().unwrap(), 0x01_u8);
        assert_eq!(producer.tell_cursor(), Offset::from(2));
        assert_eq!(producer.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(producer.tell_cursor(), Offset::from(4));
        producer.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(producer.read_u32().unwrap(), 0x00010203_u32);
        assert_eq!(producer.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(producer.tell_cursor(), Offset::from(8));
        assert!(producer.read_u64().is_err());
        producer.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(producer.read_u64().unwrap(), 0x0001020304050607_u64);
        assert_eq!(producer.tell_cursor(), Offset::from(8));

        let mut sub_producer = producer.sub_producer_at(1.into(), End::None);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(0));
        assert_eq!(sub_producer.read_u8().unwrap(), 0x01_u8);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(1));
        assert_eq!(sub_producer.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(3));
        assert_eq!(sub_producer.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(7));
        assert!(sub_producer.read_u64().is_err());
        sub_producer.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(sub_producer.read_u64().unwrap(), 0x0102030405060708_u64);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(8));

        producer.seek(SeekFrom::Start(1)).unwrap();
        sub_producer.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(producer.read_u8().unwrap(), sub_producer.read_u8().unwrap());
        assert_eq!(
            producer.read_u16().unwrap(),
            sub_producer.read_u16().unwrap()
        );
        assert_eq!(
            producer.read_u32().unwrap(),
            sub_producer.read_u32().unwrap()
        );
    }
}
