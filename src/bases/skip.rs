use std::fs::File;
use std::io;
use std::io::Seek;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub(crate) struct Skip<R> {
    inner: R,
    skip: u64,
}

impl<R> Skip<R>
where
    R: Seek,
{
    pub fn new(mut inner: R) -> io::Result<Self> {
        let skip = inner.stream_position()?;
        Ok(Self { inner, skip })
    }
}

impl<R> Skip<R> {
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl<R> io::Read for Skip<R>
where
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R> io::Write for Skip<R>
where
    R: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<R> io::Seek for Skip<R>
where
    R: io::Seek,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            io::SeekFrom::Start(s) => self.inner.seek(io::SeekFrom::Start(self.skip + s))?,
            _ => self.inner.seek(pos)?,
        };
        if new_pos < self.skip {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "You cannot seek before skip",
            ))
        } else {
            Ok(new_pos - self.skip)
        }
    }
}

impl TryFrom<File> for Skip<File> {
    type Error = io::Error;
    fn try_from(f: File) -> io::Result<Self> {
        Self::new(f)
    }
}

impl<R> Deref for Skip<R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<R> DerefMut for Skip<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_mut()
    }
}

impl AsMut<File> for Skip<File> {
    fn as_mut(&mut self) -> &mut File {
        self.inner_mut()
    }
}
