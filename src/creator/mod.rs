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

pub trait InputReader: Read + Seek + Send + 'static {
    fn size(&self) -> Size;
}

impl<T: AsRef<[u8]> + Send + 'static> InputReader for std::io::Cursor<T> {
    fn size(&self) -> Size {
        self.get_ref().as_ref().len().into()
    }
}

pub struct InputFile {
    source: std::fs::File,
    len: u64,
}

impl InputFile {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Self::new(std::fs::File::open(path)?)
    }

    pub fn new(mut source: std::fs::File) -> Result<Self> {
        let len = source.seek(SeekFrom::End(0))?;
        source.seek(SeekFrom::Start(0))?;
        Ok(Self { source, len })
    }
}

impl Seek for InputFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.source.seek(pos)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.source.rewind()
    }

    #[cfg(feature = "seek_stream_len")]
    fn stream_len(&mut self) -> std::io::Result<()> {
        self.source.stream_len()
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.source.stream_position()
    }
}

impl Read for InputFile {
    // Required method
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.source.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.source.read_vectored(bufs)
    }
    #[cfg(feature = "can_vector")]
    fn is_read_vectored(&self) -> bool {
        self.source.is_read_vectored()
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.source.read_to_end(buf)
    }
    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.source.read_to_string(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.source.read_exact(buf)
    }
    fn read_buf(&mut self, buf: std::io::BorrowedCursor<'_>) -> std::io::Result<()> {
        self.source.read_buf(buf)
    }
    fn read_buf_exact(&mut self, cursor: std::io::BorrowedCursor<'_>) -> std::io::Result<()> {
        self.source.read_buf_exact(cursor)
    }
}

impl InputReader for InputFile {
    fn size(&self) -> Size {
        self.len.into()
    }
}
