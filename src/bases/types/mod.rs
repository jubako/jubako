#[macro_use]
mod error;
mod base_array;
mod byte_size;
mod count;
mod delayed;
mod free_data;
mod id;
mod idx;
mod offset;
mod pstring;
mod range;
mod size;
mod sized_offset;
mod specific_types;

pub use base_array::BaseArray;
pub use byte_size::ByteSize;
pub use count::Count;
pub use delayed::{Bound, Late, Vow, Word};
pub use error::{Error, ErrorKind, FormatError, Result};
pub use free_data::{
    ContentPackFreeData, DirectoryPackFreeData, ManifestPackFreeData, PackInfoFreeData,
};
pub use id::Id;
pub use idx::{Idx, IndexTrait};
pub use offset::Offset;
pub use pstring::PString;
pub use range::{EntryRange, Range};
pub use size::Size;
pub use sized_offset::SizedOffset;
pub use specific_types::*;

/// The end of a buffer.
#[derive(Debug)]
pub enum End {
    Offset(Offset),
    Size(Size),
    None,
}

impl End {
    pub fn new_size<T>(s: T) -> Self
    where
        Size: From<T>,
    {
        Self::Size(Size::from(s))
    }

    pub fn new_offset<T>(o: T) -> Self
    where
        Offset: From<T>,
    {
        Self::Offset(Offset::from(o))
    }

    pub fn none() -> Self {
        End::None
    }
}
