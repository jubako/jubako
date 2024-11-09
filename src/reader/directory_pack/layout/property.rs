use super::PropertyKind;
use crate::bases::*;

/// The definition of a property, as we need to parse it.
/// In opposition to RawProperty, the property is the "final" property.
/// It describe how to parse te value of a entry.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
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

#[cfg(feature = "explorable")]
impl graphex::Display for Property {
    fn header_footer(&self) -> Option<(String, String)> {
        let header = match self.kind {
            PropertyKind::Padding => "Padding",
            PropertyKind::ContentAddress { .. } => "Content Address",
            PropertyKind::UnsignedInt { .. } => "Unsigned Int",
            PropertyKind::SignedInt { .. } => "Signed Int",
            PropertyKind::DeportedUnsignedInt { .. } => "Deported Unsigned Int",
            PropertyKind::DeportedSignedInt { .. } => "Deported Signed Int",
            PropertyKind::Array { .. } => "Array",
            PropertyKind::VariantId => "Variant Id",
        };
        Some((format!("{header}("), ")".to_string()))
    }
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        match self.kind {
            PropertyKind::Padding => {}
            PropertyKind::ContentAddress {
                pack_id_size,
                content_id_size,
                default_pack_id,
            } => {
                match default_pack_id {
                    None => {
                        out.field("offset", &self.offset.into_u64())?;
                        out.field("pack_id_size", &(pack_id_size as usize))?;
                        out.field("content_id_size", &(content_id_size as usize))?;
                    }
                    Some(pack_id) => {
                        out.field("default", &pack_id.into_u64())?;
                    }
                };
            }
            PropertyKind::UnsignedInt { int_size, default } => match default {
                None => {
                    out.field("offset", &self.offset.into_u64())?;
                    out.field("int_size", &(int_size as usize))?;
                }
                Some(int) => {
                    out.field("default", &int)?;
                }
            },
            PropertyKind::SignedInt { int_size, default } => match default {
                None => {
                    out.field("offset", &self.offset.into_u64())?;
                    out.field("int_size", &(int_size as usize))?;
                }
                Some(int) => {
                    out.field("default", &int)?;
                }
            },
            PropertyKind::DeportedUnsignedInt {
                int_size,
                value_store_idx,
                id,
            } => {
                use super::DeportedDefault;
                match id {
                    DeportedDefault::KeySize(size) => {
                        out.field("offset", &self.offset.into_u64())?;
                        out.field("int_size", &(int_size as usize))?;
                        out.field("value_store_idx", &value_store_idx.into_u64())?;
                        out.field("key_size", &(size as usize))?;
                    }
                    DeportedDefault::Value(key) => {
                        out.field("int_size", &(int_size as usize))?;
                        out.field("value_store_idx", &value_store_idx.into_u64())?;
                        out.field("key", &key)?;
                    }
                }
            }
            PropertyKind::DeportedSignedInt {
                int_size,
                value_store_idx,
                id,
            } => {
                use super::DeportedDefault;
                match id {
                    DeportedDefault::KeySize(size) => {
                        out.field("offset", &self.offset.into_u64())?;
                        out.field("int_size", &(int_size as usize))?;
                        out.field("value_store_idx", &value_store_idx.into_u64())?;
                        out.field("key_size", &(size as usize))?;
                    }
                    DeportedDefault::Value(key) => {
                        out.field("int_size", &(int_size as usize))?;
                        out.field("value_store_idx", &value_store_idx.into_u64())?;
                        out.field("key", &key)?;
                    }
                }
            }
            PropertyKind::Array {
                array_len_size,
                fixed_array_len,
                deported_info,
                default,
            } => match default {
                None => {
                    out.field("offset", &self.offset.into_u64())?;
                    out.field("array_len_size", &array_len_size.map(|s| s as usize))?;
                    out.field("base_array_len", &fixed_array_len)?;
                    out.field("deported_info", &deported_info)?;
                }
                Some((array_len, base_array, value_id)) => {
                    out.field("array_len", &array_len)?;
                    out.field("base_array", &graphex::AsBytes(&base_array.data))?;
                    out.field("deported_id", &value_id)?;
                }
            },
            PropertyKind::VariantId => {
                out.field("offset", &self.offset.into_u64())?;
            }
        }
        Ok(())
    }
}
