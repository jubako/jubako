#![feature(once_cell)]

#[macro_use]
mod bases;
pub mod creator;
mod pack;
pub mod reader;

pub use crate::bases::{Count, FreeData, Idx};
pub use reader::{CompressionType, ContentAddress};
