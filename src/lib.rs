#![cfg_attr(feature = "nightly", feature(error_generic_member_access))]

#[macro_use]
mod bases;
mod common;
pub mod creator;
pub mod reader;
pub mod tools;

pub use crate::bases::{
    Bound, ContentIdx, EntryCount, EntryIdx, EntryRange, Error, ErrorKind, FileSource, MayRef,
    Offset, PackId, PropertyCount, PropertyIdx, Reader, Result, Size, VariantIdx, VendorId, Vow,
};
pub use crate::common::{CompressionType, ContentAddress, Pack, Value};
//use crate::reader::directory_pack::layout;

#[cfg(doctest)]
use bases::BaseArray;
