#[macro_use]
mod error;
mod count;
mod free_data;
mod idx;
mod offset;
mod pstring;
mod size;
mod sized_offset;

pub use count::Count;
pub use error::{Error, FormatError, Result};
pub use free_data::FreeData;
pub use idx::{Idx, IndexTrait};
pub use offset::Offset;
pub use pstring::PString;
pub use size::Size;
pub use sized_offset::SizedOffset;

/// The end of a buffer.
pub enum End {
    Offset(Offset),
    Size(Size),
    None,
}
