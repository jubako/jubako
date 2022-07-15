#![feature(once_cell)]

#[macro_use]
mod bases;
pub mod creator;
mod pack;
pub mod reader;

pub use crate::bases::{Count, FreeData, Idx, Writable};
pub use reader::{
    Array, CompressionType, Container, Content, ContentAddress, ContentPack, DirectoryPack, Extend,
    KeyDef, KeyDefKind, MainPack, Value,
};
