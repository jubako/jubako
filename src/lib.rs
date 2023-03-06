#![feature(once_cell)]
#![feature(read_buf)]
#![feature(provide_any)]
#![feature(error_generic_member_access)]
#![feature(step_trait)]
#![feature(slice_ptr_len)]
#![feature(ptr_as_uninit)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(vec_into_raw_parts)]

#[macro_use]
mod bases;
mod common;
pub mod creator;
pub mod reader;
pub mod tools;

pub use crate::bases::{
    Bound, ContentIdx, End, EntryCount, EntryIdx, EntryRange, EntryStoreIdx, ErrorKind, FreeData31,
    FreeData40, FreeData63, Generator, Offset, PackId, PropertyCount, PropertyIdx, Reader, Result,
    Size, ValueIdx, VariantIdx, Vow, Word,
};
pub use crate::common::{CompressionType, ContentAddress, Value};
pub use crate::tools::concat;
//pub use crate::reader::directory_pack::layout;
