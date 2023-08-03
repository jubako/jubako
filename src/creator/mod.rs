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
    pub free_data: FreeData103,
    pub check_info: CheckInfo,
}
