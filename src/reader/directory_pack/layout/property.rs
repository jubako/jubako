use crate::bases::*;

// The kind of the property. This will be the descriminant to how parse the value.
#[derive(Debug, PartialEq, Eq)]
pub enum PropertyKind {
    ContentAddress,
    UnsignedInt(ByteSize),
    SignedInt(ByteSize),
    Array(usize),
    VLArray(ByteSize, ValueStoreIdx, Option<usize>),
    None,
}

/// The definition of a property, as we need to parse it.
/// In opposition to RawProperty, the property is the "final" property.
/// It describe how to parse te value of a entry.
#[derive(Debug, PartialEq, Eq)]
pub struct Property {
    pub offset: Offset,
    pub kind: PropertyKind,
}

impl Property {
    pub fn new(offset: usize, kind: PropertyKind) -> Self {
        Self {
            offset: Offset::from(offset),
            kind,
        }
    }
}
