use super::PropertyKind;
use crate::bases::*;

/// The definition of a property, as we need to parse it.
/// In opposition to RawProperty, the property is the "final" property.
/// It describe how to parse te value of a entry.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
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
