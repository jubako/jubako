#![feature(once_cell)]
#![feature(read_buf)]

#[macro_use]
mod bases;
mod common;
pub mod creator;
pub mod reader;

pub use crate::bases::{Count, FreeData, Idx};
pub use crate::common::{CompressionType, ContentAddress};
