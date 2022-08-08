#![feature(once_cell)]
#![feature(read_buf)]
#![feature(provide_any)]
#![feature(error_generic_member_access)]

#[macro_use]
mod bases;
mod common;
pub mod creator;
pub mod reader;

pub use crate::bases::{Count, FreeData, Id, Idx};
pub use crate::common::{CompressionType, ContentAddress};
