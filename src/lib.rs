#![cfg_attr(feature = "nightly", feature(error_generic_member_access))]

#[macro_use]
mod bases;
mod common;
pub mod creator;
pub mod reader;
pub mod tools;

pub use crate::bases::{
    Bound, ContentIdx, ContentPackFreeData, DirectoryPackFreeData, End, EntryCount, EntryIdx,
    EntryRange, EntryStoreIdx, ErrorKind, FileSource, IndexFreeData, ManifestPackFreeData, MayRef,
    MemoryReader, Offset, PackId, PackInfoFreeData, PropertyCount, PropertyIdx, Reader, Result,
    Size, SubReader, ValueIdx, VariantIdx, VendorId, Vow, Word,
};
pub use crate::common::{CompressionType, ContentAddress, Value};
pub use crate::tools::concat;
//pub use crate::reader::directory_pack::layout;
