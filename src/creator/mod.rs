mod basic_creator;
mod container_pack;
mod content_pack;
mod directory_pack;
mod errors;
mod manifest_pack;

pub use crate::bases::FileSource;
use crate::bases::InOutStream;
pub(crate) use crate::bases::OutStream;
use crate::bases::*;
use crate::common::{CheckInfo, CompressionType, PackKind};
pub use basic_creator::{BasicCreator, ConcatMode, EntryStoreTrait};
use camino::{Utf8Path, Utf8PathBuf};
pub use container_pack::{ContainerPackCreator, InContainerFile};
pub use content_pack::{
    CacheProgress, CachedContentAdder, CompHint, ContentAdder, ContentPackCreator, Progress,
};
pub use directory_pack::{
    schema, Array, ArrayS, DirectoryPackCreator, EntryStore, EntryTrait, ProcessedEntry,
    SimpleEntry, StoreHandle, Value, ValueHandle, ValueStore,
};
pub use errors::{Error, Result};
pub use manifest_pack::ManifestPackCreator;
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

mod private {
    use super::*;
    pub trait WritableTell {
        fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<()>;
        fn serialize_tail(&mut self, stream: &mut Serializer) -> std::io::Result<()>;
        fn write(&mut self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
            self.write_data(stream)?;
            let offset = stream.tell();
            let mut serializer = Serializer::new(BlockCheck::Crc32);
            self.serialize_tail(&mut serializer)?;
            let size = stream.write_serializer(serializer)?.into();
            Ok(SizedOffset { size, offset })
        }
    }

    pub trait Sealed {}

    pub enum MaybeFileReader {
        Yes(std::io::Take<std::fs::File>),
        No(Box<dyn Read + Send>),
    }
}

pub struct PackData {
    pub uuid: uuid::Uuid,
    pub pack_size: Size,
    pub pack_kind: PackKind,
    pub pack_id: PackId,
    pub free_data: Vec<u8>,
    pub check_info: CheckInfo,
}

pub(crate) use private::MaybeFileReader;

pub trait InputReader: Read + Seek + Send + 'static {
    fn size(&self) -> Size;
    fn get_file_source(self: Box<Self>) -> MaybeFileReader;
}

impl<T: AsRef<[u8]> + Send + 'static> InputReader for std::io::Cursor<T> {
    fn size(&self) -> Size {
        self.get_ref().as_ref().len().into()
    }
    fn get_file_source(self: Box<Self>) -> MaybeFileReader {
        MaybeFileReader::No(self)
    }
}

pub struct InputFile {
    source: std::fs::File,
    position: u64,
    origin: u64,
    len: u64,
}

impl InputFile {
    pub fn open(path: impl AsRef<Path>) -> IoResult<Self> {
        Self::new(std::fs::File::open(path)?)
    }

    pub fn new(source: std::fs::File) -> IoResult<Self> {
        Self::new_range(source, 0, None)
    }

    pub fn new_range(mut source: std::fs::File, origin: u64, size: Option<u64>) -> IoResult<Self> {
        let total_len = source.seek(SeekFrom::End(0))?;
        let size = match size {
            None => total_len - origin,
            Some(s) => s,
        };
        source.seek(SeekFrom::Start(origin))?;
        Ok(Self {
            source,
            position: origin,
            origin,
            len: size,
        })
    }

    fn local_position(&self) -> u64 {
        self.position - self.origin
    }
}

impl Seek for InputFile {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        let pos = match pos {
            SeekFrom::Start(o) => SeekFrom::Start(self.origin + o),
            SeekFrom::Current(o) => SeekFrom::Current(o),
            SeekFrom::End(e) => SeekFrom::Start((self.origin as i64 + self.len as i64 + e) as u64),
        };
        self.position = self.source.seek(pos)?;
        Ok(self.position)
    }

    fn rewind(&mut self) -> IoResult<()> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    #[cfg(feature = "nightly")]
    fn stream_len(&mut self) -> IoResult<()> {
        Ok(self.len)
    }

    fn stream_position(&mut self) -> IoResult<u64> {
        Ok(self.position - self.origin)
    }
}

impl Read for InputFile {
    // Required method
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let size_to_read = std::cmp::min(
            buf.len(),
            (self.len - self.local_position()).try_into().unwrap(),
        );
        let actually_read = self.source.read(&mut buf[..size_to_read])?;
        self.position += actually_read as u64;
        Ok(actually_read)
    }
}

impl InputReader for InputFile {
    fn size(&self) -> Size {
        self.len.into()
    }

    fn get_file_source(mut self: Box<Self>) -> MaybeFileReader {
        self.rewind().unwrap();
        MaybeFileReader::Yes(self.source.take(self.len))
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Compression {
    None,
    #[cfg(feature = "lz4")]
    Lz4(deranged::RangedU32<0, 15>),

    #[cfg(feature = "lzma")]
    Lzma(deranged::RangedU32<0, 9>),

    #[cfg(feature = "zstd")]
    Zstd(deranged::RangedI32<-22, 22>),
}

impl Default for Compression {
    #[cfg(feature = "zstd")]
    fn default() -> Self {
        Compression::zstd()
    }

    #[cfg(all(feature = "lzma", not(feature = "zstd")))]
    fn default() -> Self {
        Compression::lzma()
    }

    #[cfg(all(feature = "lz4", not(feature = "lzma"), not(feature = "zstd")))]
    fn default() -> Self {
        Compression::lz4()
    }

    #[cfg(all(not(feature = "lz4"), not(feature = "lzma"), not(feature = "zstd")))]
    fn default() -> Self {
        Compression::None
    }
}

impl Compression {
    #[cfg(feature = "lz4")]
    pub fn lz4() -> Compression {
        Compression::Lz4(deranged::RangedU32::new_static::<3>())
    }

    #[cfg(feature = "lzma")]
    pub fn lzma() -> Compression {
        Compression::Lzma(deranged::RangedU32::new_static::<9>())
    }

    #[cfg(feature = "zstd")]
    pub fn zstd() -> Compression {
        Compression::Zstd(deranged::RangedI32::new_static::<5_i32>())
    }
}

impl From<Compression> for CompressionType {
    fn from(c: Compression) -> Self {
        match c {
            Compression::None => CompressionType::None,
            #[cfg(feature = "lz4")]
            Compression::Lz4(_) => CompressionType::Lz4,
            #[cfg(feature = "lzma")]
            Compression::Lzma(_) => CompressionType::Lzma,
            #[cfg(feature = "zstd")]
            Compression::Zstd(_) => CompressionType::Zstd,
        }
    }
}

/// Something on which we can write a Pack.
/// This may be a File or not.
pub trait PackRecipient: InOutStream + private::Sealed {
    fn close_file(self: Box<Self>) -> Result<Utf8PathBuf>;
}

#[derive(Debug)]
pub struct NamedFile {
    file: std::fs::File,
    final_path: Utf8PathBuf,
}

impl NamedFile {
    fn new<P: AsRef<Utf8Path>>(final_path: P) -> IoResult<Box<Self>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(final_path.as_ref().as_std_path())?;
        Ok(Box::new(Self {
            file,
            final_path: final_path.as_ref().into(),
        }))
    }

    #[inline]
    pub fn into_inner(self) -> std::fs::File {
        self.file
    }
}

impl Seek for NamedFile {
    fn seek(&mut self, pos: io::SeekFrom) -> IoResult<u64> {
        self.file.seek(pos)
    }
}

impl io::Write for NamedFile {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.file.flush()
    }
}

impl io::Read for NamedFile {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.file.read(buf)
    }
}

impl OutStream for NamedFile {
    fn copy(
        &mut self,
        reader: Box<dyn crate::creator::InputReader>,
    ) -> IoResult<(u64, MaybeFileReader)> {
        self.file.copy(reader)
    }
}

impl private::Sealed for NamedFile {}

impl PackRecipient for NamedFile {
    fn close_file(self: Box<Self>) -> Result<Utf8PathBuf> {
        Ok(self.final_path)
    }
}

#[derive(Debug)]
pub struct AtomicOutFile {
    temp_file: tempfile::NamedTempFile,
    final_path: Utf8PathBuf,
}

impl AtomicOutFile {
    pub fn new<P: AsRef<Utf8Path>>(final_path: P) -> IoResult<Box<Self>> {
        let final_path = camino::absolute_utf8(final_path.as_ref())?;
        let parent = final_path.parent().unwrap();
        let temp_file = tempfile::NamedTempFile::new_in(parent)?;
        Ok(Box::new(Self {
            temp_file,
            final_path: final_path.to_path_buf(),
        }))
    }
}

impl Seek for AtomicOutFile {
    fn seek(&mut self, pos: io::SeekFrom) -> IoResult<u64> {
        self.temp_file.seek(pos)
    }
}

impl io::Write for AtomicOutFile {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.temp_file.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.temp_file.flush()
    }
}

impl io::Read for AtomicOutFile {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.temp_file.read(buf)
    }
}

impl OutStream for AtomicOutFile {
    fn copy(
        &mut self,
        reader: Box<dyn crate::creator::InputReader>,
    ) -> IoResult<(u64, MaybeFileReader)> {
        self.temp_file.as_file_mut().copy(reader)
    }
}

impl private::Sealed for AtomicOutFile {}
impl PackRecipient for AtomicOutFile {
    fn close_file(self: Box<Self>) -> Result<Utf8PathBuf> {
        self.temp_file
            .persist(&self.final_path)
            .map_err(|e| e.error)?;
        Ok(self.final_path)
    }
}
