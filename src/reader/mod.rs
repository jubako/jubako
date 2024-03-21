#[macro_use]
mod content_pack;
mod container_pack;
mod directory_pack;
mod jubako;
mod locator;
mod manifest_pack;
mod missing;

pub use self::jubako::Container;
pub use container_pack::ContainerPack;
pub use content_pack::ContentPack;
pub use directory_pack::{builder, layout};
pub use missing::MayMissPack;
pub type EntryStore = std::sync::Arc<directory_pack::EntryStore>;
pub use crate::common::{ManifestPackHeader, PackInfo};
pub use directory_pack::{
    Array, CompareTrait, ContentAddress, DirectoryPack, EntryTrait, Index, LazyEntry as Entry,
    PropertyCompare, RangeTrait as Range, RawValue, Value, ValueStorage,
};
pub use layout::VariantPart;
pub use locator::{ChainedLocator, FsLocator, PackLocatorTrait};
pub use manifest_pack::ManifestPack;

pub mod testing {
    pub use super::directory_pack::Extend;
}

#[cfg(feature = "explorable")]
pub use crate::bases::Explorable;
pub use crate::bases::Producable;
