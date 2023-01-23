#[macro_use]
mod error;
mod byte_size;
mod count;
mod delayed;
mod free_data;
mod id;
mod idx;
mod offset;
mod pstring;
mod size;
mod sized_offset;
mod specific_types;

pub use byte_size::ByteSize;
pub use count::Count;
pub use delayed::{Bound, Late, Vow, Word};
pub use error::{Error, ErrorKind, FormatError, Result};
pub use free_data::{FreeData103, FreeData31, FreeData40, FreeData63};
pub use id::Id;
pub use idx::{Idx, IndexTrait};
pub use offset::Offset;
pub use pstring::PString;
pub use size::Size;
pub use sized_offset::SizedOffset;
pub use specific_types::*;

/// The end of a buffer.
pub enum End {
    Offset(Offset),
    Size(Size),
    None,
}

impl End {
    pub fn new_size(s: u64) -> Self {
        Self::Size(Size::from(s))
    }

    pub fn new_offset(o: u64) -> Self {
        Self::Offset(Offset::from(o))
    }

    pub fn none() -> Self {
        End::None
    }
}
