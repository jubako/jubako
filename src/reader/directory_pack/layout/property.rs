use super::PropertyKind;
use crate::bases::*;

/// The definition of a property, as we need to parse it.
/// In opposition to RawProperty, the property is the "final" property.
/// It describe how to parse te value of a entry.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize))]
pub struct Property {
    pub(crate) offset: Offset,
    pub(crate) kind: PropertyKind,
}

impl Property {
    pub(crate) fn new(offset: usize, kind: PropertyKind) -> Self {
        Self {
            offset: Offset::from(offset),
            kind,
        }
    }
}
