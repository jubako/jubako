mod buffer;
mod compression;
mod file;

use crate::bases::stream::*;
use crate::bases::types::*;
pub use buffer::*;
pub use compression::*;
pub use file::*;
use std::io::{ErrorKind, Seek, SeekFrom};
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

impl<T> Seek for StreamWrapper<T> {
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
