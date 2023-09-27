mod container_pack;
mod content_pack;
mod directory_pack;
mod manifest_pack;

use crate::bases::*;
pub use crate::bases::{FileSource, Reader};
use crate::common::{CheckInfo, PackKind};
pub use container_pack::ContainerPackCreator;
pub use content_pack::{CacheProgress, CachedContentPackCreator, ContentPackCreator, Progress};
pub use directory_pack::{
    schema, BasicEntry, DirectoryPackCreator, EntryStore, EntryTrait, IndexedValueStore,
    PlainValueStore, PropertyName, Value, ValueStore, ValueTransformer, VariantName,
};
pub use manifest_pack::ManifestPackCreator;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub enum Embedded {
    Yes,
    No(PathBuf),
}

mod private {
    use super::*;
    pub trait WritableTell {
        fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<()>;
        fn write_tail(&mut self, stream: &mut dyn OutStream) -> Result<()>;
        fn write(&mut self, stream: &mut dyn OutStream) -> Result<SizedOffset> {
            self.write_data(stream)?;
            let offset = stream.tell();
            self.write_tail(stream)?;
            let size = stream.tell() - offset;
            Ok(SizedOffset { size, offset })
        }
    }
}

pub struct PackData {
    pub uuid: uuid::Uuid,
    pub pack_size: Size,
    pub pack_kind: PackKind,
    pub pack_id: PackId,
    pub free_data: PackInfoFreeData,
    pub check_info: CheckInfo,
}

pub enum MaybeFileReader {
    Yes(std::io::Take<std::fs::File>),
    No(Box<dyn Read>),
}

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
    pub(crate) source: std::fs::File,
    position: u64,
    origin: u64,
    len: u64,
}

impl InputFile {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Self::new(std::fs::File::open(path)?)
    }

    pub fn new(source: std::fs::File) -> Result<Self> {
        Self::new_range(source, 0, None)
    }

    pub fn new_range(mut source: std::fs::File, origin: u64, size: Option<u64>) -> Result<Self> {
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
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let pos = match pos {
            SeekFrom::Start(o) => SeekFrom::Start(self.origin + o),
            SeekFrom::Current(o) => SeekFrom::Current(o),
            SeekFrom::End(e) => SeekFrom::Start((self.origin as i64 + self.len as i64 + e) as u64),
        };
        self.position = self.source.seek(pos)?;
        Ok(self.position)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    #[cfg(feature = "seek_stream_len")]
    fn stream_len(&mut self) -> std::io::Result<()> {
        Ok(self.len)
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.position - self.origin)
    }
}

impl Read for InputFile {
    // Required method
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
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
