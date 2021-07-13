use crate::bases::producing::*;
use crate::bases::types::*;
use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::rc::Rc;

pub struct ProducerWrapper<T> {
    source: Rc<T>,
    origin: Offset,
    end: Offset,
    offset: Offset,
}

impl<T> ProducerWrapper<T> {
    pub fn new_from_parts(source: Rc<T>, origin: Offset, end: Offset, offset: Offset) -> Self {
        Self {
            source,
            origin,
            end,
            offset,
        }
    }
}

impl ProducerWrapper<Vec<u8>> {
    fn slice(&self) -> &[u8] {
        let offset = self.offset.0 as usize;
        let end = self.end.0 as usize;
        &self.source[offset..end]
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
    fn skip(&mut self, size: Size) -> Result<()> {
        let new_offset = self.offset + size;
        if new_offset <= self.end {
            self.offset = new_offset;
            Ok(())
        } else {
            Err(Error::FormatError)
        }
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

pub struct SeekableDecoder<T> {
    decoder: RefCell<T>,
    buffer: RefCell<Box<[u8]>>,
    decoded: Cell<Offset>,
}

impl<T: Read> SeekableDecoder<T> {
    pub fn new(decoder: T, size: Size) -> Self {
        let mut buffer = Vec::with_capacity(size.0 as usize);
        unsafe {
            buffer.set_len(size.0 as usize);
        }
        Self {
            decoder: RefCell::new(decoder),
            buffer: RefCell::new(buffer.into()),
            decoded: Cell::new(Offset(0)),
        }
    }

    pub fn decode_to(&self, end: Offset) -> std::result::Result<(), std::io::Error> {
        if end >= self.decoded.get() {
            let o = self.decoded.get().0 as usize;
            let e = std::cmp::min(end.0 as usize, self.buffer.borrow().len());
            self.decoder
                .borrow_mut()
                .read_exact(&mut self.buffer.borrow_mut()[o..e])?;
            self.decoded.set(Offset::from(e as u64));
        }
        Ok(())
    }

    pub fn decoded_slice(&self) -> &[u8] {
        let size = self.decoded.get().0 as usize;
        assert!(size <= self.buffer.borrow().len());
        let ptr = self.buffer.borrow().as_ptr();
        unsafe { std::slice::from_raw_parts(ptr, size) }
    }
}

impl<T: Read> ProducerWrapper<SeekableDecoder<T>> {
    fn slice(&self) -> &[u8] {
        let o = self.offset.0 as usize;
        let e = self.end.0 as usize;
        &self.source.decoded_slice()[o..e]
    }
}

impl<T: Read> Read for ProducerWrapper<SeekableDecoder<T>> {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let end = self.offset + buf.len();
        self.source.decode_to(end)?;
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
    use std::io::Write;
    use tempfile::tempfile;

    #[test]
    fn test_vec_producer() {
        let reader = BufReader::new(
            vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            End::None,
        );
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(producer.read_u8().unwrap(), 0x00_u8);
        assert_eq!(producer.tell_cursor(), Offset::from(1));
        assert_eq!(producer.read_u8().unwrap(), 0x01_u8);
        assert_eq!(producer.tell_cursor(), Offset::from(2));
        assert_eq!(producer.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(producer.tell_cursor(), Offset::from(4));
        producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(producer.read_u32().unwrap(), 0x00010203_u32);
        assert_eq!(producer.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(producer.tell_cursor(), Offset::from(8));
        assert!(producer.read_u64().is_err());
        producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(producer.read_u64().unwrap(), 0x0001020304050607_u64);
        assert_eq!(producer.tell_cursor(), Offset::from(8));

        let mut sub_producer = reader.create_stream(1.into(), End::None);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(0));
        assert_eq!(sub_producer.read_u8().unwrap(), 0x01_u8);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(1));
        assert_eq!(sub_producer.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(3));
        assert_eq!(sub_producer.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(7));
        assert!(sub_producer.read_u64().is_err());
        sub_producer = reader.create_stream(1.into(), End::None);
        assert_eq!(sub_producer.read_u64().unwrap(), 0x0102030405060708_u64);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(8));

        producer = reader.create_stream(Offset(0), End::None);
        sub_producer = reader.create_stream(1.into(), End::None);
        producer.skip(Size(1)).unwrap();
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
        let reader = FileReader::new(file, End::None);
        let mut producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(producer.read_u8().unwrap(), 0x00_u8);
        assert_eq!(producer.tell_cursor(), Offset::from(1));
        assert_eq!(producer.read_u8().unwrap(), 0x01_u8);
        assert_eq!(producer.tell_cursor(), Offset::from(2));
        assert_eq!(producer.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(producer.tell_cursor(), Offset::from(4));
        producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(producer.read_u32().unwrap(), 0x00010203_u32);
        assert_eq!(producer.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(producer.tell_cursor(), Offset::from(8));
        assert!(producer.read_u64().is_err());
        producer = reader.create_stream(Offset(0), End::None);
        assert_eq!(producer.read_u64().unwrap(), 0x0001020304050607_u64);
        assert_eq!(producer.tell_cursor(), Offset::from(8));

        let mut sub_producer = reader.create_stream(Offset(1), End::None);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(0));
        assert_eq!(sub_producer.read_u8().unwrap(), 0x01_u8);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(1));
        assert_eq!(sub_producer.read_u16().unwrap(), 0x0203_u16);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(3));
        assert_eq!(sub_producer.read_u32().unwrap(), 0x04050607_u32);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(7));
        assert!(sub_producer.read_u64().is_err());
        sub_producer = reader.create_stream(Offset(1), End::None);
        assert_eq!(sub_producer.read_u64().unwrap(), 0x0102030405060708_u64);
        assert_eq!(sub_producer.tell_cursor(), Offset::from(8));

        producer = reader.create_stream(Offset(0), End::None);
        sub_producer = reader.create_stream(Offset(1), End::None);
        producer.skip(Size(1)).unwrap();
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
