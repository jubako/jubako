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
                        out.item("offset", &self.offset.into_u64())?;
                        out.item("pack_id_size", &(pack_id_size as usize))?;
                        out.item("content_id_size", &(content_id_size as usize))?;
                    }
                    Some(pack_id) => {
                        out.item("default", &pack_id.into_u64())?;
                    }
                };
            }
            PropertyKind::UnsignedInt { int_size, default } => match default {
                None => {
                    out.item("offset", &self.offset.into_u64())?;
                    out.item("int_size", &(int_size as usize))?;
                }
                Some(int) => {
                    out.item("default", &int)?;
                }
            },
            PropertyKind::SignedInt { int_size, default } => match default {
                None => {
                    out.item("offset", &self.offset.into_u64())?;
                    out.item("int_size", &(int_size as usize))?;
                }
                Some(int) => {
                    out.item("default", &int)?;
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
                        out.item("offset", &self.offset.into_u64())?;
                        out.item("int_size", &(int_size as usize))?;
                        out.item("value_store_idx", &value_store_idx.into_u64())?;
                        out.item("key_size", &(size as usize))?;
                    }
                    DeportedDefault::Value(key) => {
                        out.item("int_size", &(int_size as usize))?;
                        out.item("value_store_idx", &value_store_idx.into_u64())?;
                        out.item("key", &key)?;
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
                        out.item("offset", &self.offset.into_u64())?;
                        out.item("int_size", &(int_size as usize))?;
                        out.item("value_store_idx", &value_store_idx.into_u64())?;
                        out.item("key_size", &(size as usize))?;
                    }
                    DeportedDefault::Value(key) => {
                        out.item("int_size", &(int_size as usize))?;
                        out.item("value_store_idx", &value_store_idx.into_u64())?;
                        out.item("key", &key)?;
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
                    out.item("offset", &self.offset.into_u64())?;
                    out.item("array_len_size", &array_len_size.map(|s| s as usize))?;
                    out.item("base_array_len", &fixed_array_len)?;
                    out.item("deported_info", &deported_info)?;
                }
                Some((array_len, base_array, value_id)) => {
                    out.item("array_len", &array_len)?;
                    out.item("base_array", &graphex::AsBytes(&base_array.data))?;
                    out.item("deported_id", &value_id)?;
                }
            },
            PropertyKind::VariantId => {
                out.item("offset", &self.offset.into_u64())?;
            }
        }
        Ok(())
    }
}
