#![cfg_attr(feature = "nightly", feature(error_generic_member_access))]

#[macro_use]
mod bases;
mod common;
pub mod creator;
pub mod reader;
pub mod tools;

pub use crate::bases::{
    Bound, ContentIdx, ContentPackFreeData, DirectoryPackFreeData, End, EntryCount, EntryIdx,
    EntryRange, EntryStoreIdx, ErrorKind, FileSource, Flux, IndexFreeData, ManifestPackFreeData,
    MayRef, MemoryReader, Offset, PString, PackId, PackInfoFreeData, PropertyCount, PropertyIdx,
    Reader, Result, Size, SizedProducable, Stream, SubReader, ValueIdx, VariantIdx, VendorId, Vow,
    Word, Writable,
};
pub use crate::common::{
    CheckInfo, CompressionType, ContainerPackHeader, ContentAddress, PackKind, Value,
};
pub use crate::tools::concat;
//pub use crate::reader::directory_pack::layout;
