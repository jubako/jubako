use crate::bases::*;

// The kind of the key. This will be the descriminant to how parse the value.
#[derive(Debug, PartialEq)]
pub enum KeyKind {
    ContentAddress(u8),
    UnsignedInt(usize),
    SignedInt(usize),
    CharArray(usize),
    PString(usize, Idx<u8>, Option<Box<Key>>),
    None,
}

/// The definition of a key, as we need to parse it.
/// In opposition to KeyDef, the key is the "final" key.
/// It describe how to parse te value of a entry.
#[derive(Debug, PartialEq)]
pub struct Key {
    offset: Offset,
    pub kind: KeyKind,
}

impl Key {
    pub fn new(offset: usize, kind: KeyKind) -> Self {
        Self {
            offset: Offset(offset as u64),
            kind,
        }
    }
}
