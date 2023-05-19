#[macro_use]
mod content_pack;
mod directory_pack;
mod jubako;
mod manifest_pack;

pub use self::jubako::Container;
pub use content_pack::ContentPack;
pub use directory_pack::{builder, layout};
pub type EntryStore = std::sync::Arc<directory_pack::EntryStore>;
pub use directory_pack::{
    Array, CompareTrait, ContentAddress, DirectoryPack, EntryTrait, Index, LazyEntry as Entry,
    PropertyCompare, RangeTrait as Range, RawValue, Value, ValueStorage,
};
pub use manifest_pack::ManifestPack;

pub mod testing {
    pub use super::directory_pack::Extend;
}
