#![feature(once_cell)]
#![feature(read_buf)]
#![feature(provide_any)]
#![feature(error_generic_member_access)]
#![feature(step_trait)]

#[macro_use]
mod bases;
mod common;
pub mod creator;
pub mod reader;
pub mod tools;

pub use crate::bases::{
    ContentIdx, End, EntryCount, EntryIdx, EntryStoreIdx, FreeData31, FreeData40, FreeData63,
    Offset, PackId, PropertyIdx, Result, Size, ValueIdx, Reader
};
pub use crate::common::{CompressionType, ContentAddress};
pub use crate::tools::concat;
//pub use crate::reader::directory_pack::layout;
