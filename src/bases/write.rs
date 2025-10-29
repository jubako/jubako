use crate::bases::*;
use crate::creator::MaybeFileReader;
use std::fmt::Debug;
use std::io::{Read, Seek, Write};

pub(crate) mod private {
    use super::primitive;
    use super::{BlockCheck, ByteSize, MaybeFileReader, Offset, CRC};
    use std::fmt::Debug;
    use std::io::{Cursor, Read, Result as IoResult, Seek, Write};

    /// A Serializable is a object we can serialized on a [Serializer].
    pub trait Serializable {
        fn serialize(&self, stream: &mut Serializer) -> IoResult<usize>;
    }

    /// A Buffer on which we can write data.
    pub struct Serializer {
        buf: Cursor<Vec<u8>>,
        check: BlockCheck,
    }

    impl Serializer {
        pub fn new(check: BlockCheck) -> Self {
            Self {
                buf: Cursor::new(Vec::with_capacity(256)),
                check,
            }
        }

        pub fn close(self) -> (Vec<u8>, Option<[u8; 4]>) {
            match self.check {
                BlockCheck::None => (self.buf.into_inner(), None),
                BlockCheck::Crc32 => {
                    let buf = self.buf.into_inner();
                    let mut digest = CRC.digest();
                    digest.update(&buf);
                    let checksum = digest.finalize();
                    let checksum = checksum.to_be_bytes();

                    #[cfg(debug_assertions)]
                    {
                        let mut digest = CRC.digest();
                        digest.update(&buf);
                        digest.update(&checksum);
                        assert_eq!(digest.finalize(), 0);
                    }
                    (buf, Some(checksum))
                }
            }
        }

        pub fn len(&self) -> usize {
            self.buf.get_ref().len()
        }
    }

    impl Serializer {
        pub fn write_u8(&mut self, value: u8) -> IoResult<usize> {
            self.buf.write_all(&primitive::write_u8(value))?;
            Ok(1)
        }
        pub fn write_u16(&mut self, value: u16) -> IoResult<usize> {
            self.buf.write_all(&primitive::write_u16(value))?;
            Ok(2)
        }
        pub fn write_u32(&mut self, value: u32) -> IoResult<usize> {
            self.buf.write_all(&primitive::write_u32(value))?;
            Ok(4)
        }
        pub fn write_u64(&mut self, value: u64) -> IoResult<usize> {
            self.buf.write_all(&primitive::write_u64(value))?;
            Ok(8)
        }
        pub fn write_usized(&mut self, value: u64, size: ByteSize) -> IoResult<usize> {
            let size = size as usize;
            self.buf
                .write_all(&primitive::write_from_u64(value, size)[..size])?;
            Ok(size)
        }
        pub fn write_isized(&mut self, value: i64, size: ByteSize) -> IoResult<usize> {
            let size = size as usize;
            self.buf
                .write_all(&primitive::write_from_i64(value, size)[..size])?;
            Ok(size)
        }
        pub fn write_data(&mut self, buf: &[u8]) -> IoResult<usize> {
            self.buf.write_all(buf)?;
            Ok(buf.len())
        }
    }

    /// A OutStream is a object on which we can write data.
    pub trait OutStream: Write + Seek + Send + Debug {
        fn copy(
            &mut self,
            reader: Box<dyn crate::creator::InputReader>,
        ) -> IoResult<(u64, MaybeFileReader)>;

        fn tell(&mut self) -> Offset {
            self.stream_position().unwrap().into()
        }

        fn write_serializer(&mut self, serializer: Serializer) -> IoResult<usize> {
            let (data, check) = serializer.close();
            self.write_all(&data)?;
            if let Some(check) = check {
                self.write_all(&check)?;
            }
            Ok(data.len())
        }

        fn ser_write(&mut self, obj: &dyn Serializable) -> IoResult<usize> {
            let mut serializer = Serializer::new(BlockCheck::Crc32);
            let written = obj.serialize(&mut serializer)?;
            assert_eq!(written, serializer.len());
            self.write_serializer(serializer)
        }

        fn ser_callable(
            &mut self,
            fun: &dyn Fn(&mut Serializer) -> IoResult<()>,
        ) -> IoResult<usize> {
            let mut serializer = Serializer::new(BlockCheck::Crc32);
            fun(&mut serializer)?;
            self.write_serializer(serializer)
        }
    }

    pub trait InOutStream: OutStream + Read {}
}

pub(crate) use private::{InOutStream, OutStream, Serializable, Serializer};

impl OutStream for std::fs::File {
    fn copy(
        &mut self,
        reader: Box<dyn crate::creator::InputReader>,
    ) -> IoResult<(u64, MaybeFileReader)> {
        let mut maybe_file_reader = reader.get_file_source();
        let read = match maybe_file_reader {
            crate::creator::MaybeFileReader::Yes(ref mut input_file) => {
                std::io::copy(input_file, self)?
            }
            crate::creator::MaybeFileReader::No(ref mut reader) => {
                std::io::copy(reader.as_mut(), self)?
            }
        };
        Ok((read, maybe_file_reader))
    }
}

impl<T> OutStream for std::io::Cursor<T>
where
    std::io::Cursor<T>: Write + Seek + Send + std::fmt::Debug,
{
    fn copy(
        &mut self,
        reader: Box<dyn crate::creator::InputReader>,
    ) -> IoResult<(u64, MaybeFileReader)> {
        let mut maybe_file_reader = reader.get_file_source();
        let read = match maybe_file_reader {
            crate::creator::MaybeFileReader::Yes(ref mut input_file) => {
                std::io::copy(input_file, self)?
            }
            crate::creator::MaybeFileReader::No(ref mut reader) => {
                std::io::copy(reader.as_mut(), self)?
            }
        };
        Ok((read, maybe_file_reader))
    }
}

impl<T> OutStream for std::io::BufWriter<T>
where
    T: Write + Seek + Send + Debug,
{
    fn copy(
        &mut self,
        reader: Box<dyn crate::creator::InputReader>,
    ) -> IoResult<(u64, MaybeFileReader)> {
        let mut maybe_file_reader = reader.get_file_source();
        let read = match maybe_file_reader {
            crate::creator::MaybeFileReader::Yes(ref mut input_file) => {
                std::io::copy(input_file, self)?
            }
            crate::creator::MaybeFileReader::No(ref mut reader) => {
                std::io::copy(reader.as_mut(), self)?
            }
        };
        Ok((read, maybe_file_reader))
    }
}

impl<O> OutStream for Skip<O>
where
    O: OutStream,
{
    fn copy(
        &mut self,
        reader: Box<dyn crate::creator::InputReader>,
    ) -> IoResult<(u64, MaybeFileReader)> {
        self.inner_mut().copy(reader)
    }
}

impl<O> OutStream for Box<O>
where
    O: OutStream + ?Sized,
{
    fn copy(
        &mut self,
        reader: Box<dyn crate::creator::InputReader>,
    ) -> IoResult<(u64, MaybeFileReader)> {
        self.as_mut().copy(reader)
    }
}

impl<T> InOutStream for T where T: OutStream + Read {}
