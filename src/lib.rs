#![feature(once_cell)]
#![feature(read_buf)]
#![feature(provide_any)]
#![feature(error_generic_member_access)]

#[macro_use]
mod bases;
mod common;
pub mod creator;
pub mod reader;
pub mod tools;

pub use crate::bases::{Count, End, FreeData, Id, Idx, Result};
pub use crate::common::{CompressionType, ContentAddress};
pub use crate::tools::concat;
