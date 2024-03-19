use super::super::PropertyName;
use super::StoreHandle;
use crate::bases::Writable;
use crate::bases::*;

pub enum Property<PN: PropertyName> {
    VariantId(String),
    Array {
        array_len_size: Option<ByteSize>,
        fixed_array_len: u8,
        deported_info: Option<(ByteSize, StoreHandle)>,
        name: PN,
    },
    IndirectArray {
        value_id_size: ByteSize,
        store_handle: StoreHandle,
        name: PN,
    },
    ContentAddress {
        content_id_size: ByteSize,
        pack_id_size: ByteSize,
        default: Option<u16>,
        name: PN,
    },
    UnsignedInt {
        size: ByteSize,
        default: Option<u64>,
        name: PN,
    },
    SignedInt {
        size: ByteSize,
        default: Option<i64>,
        name: PN,
    },
    Padding(/*size*/ u8),
}

impl<PN: PropertyName> std::fmt::Debug for Property<PN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Property::*;
        match self {
            VariantId(name) => f
                .debug_struct("VariantId")
                .field("name", &name)
                .field("size", &self.size())
                .finish(),
            Array {
                array_len_size,
                fixed_array_len,
                deported_info,
                name,
            } => f
                .debug_struct("Array")
                .field("array_len_size", &array_len_size)
                .field("fixed_array_len", &fixed_array_len)
                .field("deported_info", &deported_info)
                .field("size", &self.size())
                .field("name", &name.to_string())
                .finish(),
            IndirectArray {
                value_id_size,
                store_handle,
                name,
            } => f
                .debug_struct("IndirectArray")
                .field("value_id_size", &value_id_size)
                .field("store_handle", &store_handle)
                .field("name", &name.to_string())
                .finish(),
            ContentAddress {
                content_id_size,
                pack_id_size,
                default,
                name,
            } => f
                .debug_struct("ContentAddress")
                .field("content_id_size", &content_id_size)
                .field("pack_id_size", &pack_id_size)
                .field("default", &default)
                .field("size", &self.size())
                .field("name", &name.to_string())
                .finish(),
            UnsignedInt {
                size,
                default,
                name,
            } => f
                .debug_struct("UnsignedInt")
                .field("size", &size)
                .field("default", &default)
                .field("size", &self.size())
                .field("name", &name.to_string())
                .finish(),
            SignedInt {
                size,
                default,
                name,
            } => f
                .debug_struct("SignedInt")
                .field("size", &size)
                .field("default", &default)
                .field("size", &self.size())
                .field("name", &name.to_string())
                .finish(),
            Padding(_size) => f
                .debug_struct("Padding")
                .field("size", &self.size())
                .finish(),
        }
    }
}

impl<PN: PropertyName> Property<PN> {
    pub(crate) fn size(&self) -> u16 {
        match self {
            Property::VariantId(_name) => 1,
            Property::Array {
                array_len_size,
                fixed_array_len,
                deported_info,
                name: _,
            } => {
                (match array_len_size {
                    None => 0,
                    Some(s) => *s as usize as u16,
                }) + *fixed_array_len as u16
                    + match deported_info {
                        None => 0,
                        Some((s, _)) => *s as usize as u16,
                    }
            }
            Property::IndirectArray {
                value_id_size,
                store_handle: _,
                name: _,
            } => *value_id_size as usize as u16,
            Property::ContentAddress {
                content_id_size,
                pack_id_size,
                default,
                name: _,
            } => {
                (if default.is_some() {
                    0
                } else {
                    *pack_id_size as u16
                }) + *content_id_size as u16
            }
            Property::UnsignedInt {
                size,
                default,
                name: _,
            } => {
                if default.is_some() {
                    0
                } else {
                    *size as u16
                }
            }
            Property::SignedInt {
                size,
                default,
                name: _,
            } => {
                if default.is_some() {
                    0
                } else {
                    *size as u16
                }
            }
            Property::Padding(size) => *size as u16,
        }
    }
}

impl<PN: PropertyName> Writable for Property<PN> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        match self {
            Property::VariantId(name) => {
                let mut written = stream.write_u8(PropType::VariantId as u8)?;
                written += PString::write_string(name.as_bytes(), stream)?;
                Ok(written)
            }
            Property::Array {
                array_len_size,
                fixed_array_len,
                deported_info,
                name,
            } => {
                let mut written = 0;
                let keytype = PropType::Array as u8
                    + match array_len_size {
                        None => 0,
                        Some(s) => *s as usize as u8,
                    };
                written += stream.write_u8(keytype)?;
                let key_size = match deported_info {
                    None => 0,
                    Some((s, _)) => *s as usize as u8,
                } << 5;
                written += stream.write_u8(key_size + fixed_array_len)?;
                if let Some((_, store)) = deported_info {
                    written += store.get_idx().unwrap().write(stream)?;
                }
                written += PString::write_string(name.to_string().as_bytes(), stream)?;
                Ok(written)
            }
            Property::IndirectArray {
                value_id_size,
                store_handle,
                name,
            } => {
                let mut written = stream.write_u8(PropType::Array as u8)?;
                written += stream.write_u8((*value_id_size as usize as u8) << 5)?;
                written += store_handle.get_idx().unwrap().write(stream)?;
                written += PString::write_string(name.to_string().as_bytes(), stream)?;
                Ok(written)
            }
            Property::ContentAddress {
                content_id_size,
                pack_id_size,
                default,
                name,
            } => {
                let mut key_type = PropType::ContentAddress as u8;
                key_type += *content_id_size as u8 - 1;
                if let ByteSize::U2 = pack_id_size {
                    key_type |= 0b0000_0100;
                }
                let mut written = match default {
                    None => stream.write_u8(key_type)?,
                    Some(d) => {
                        let mut written = 0;
                        written += stream.write_u8(key_type + 0b0000_1000)?;
                        written += stream.write_usized(*d as u64, *pack_id_size)?;
                        written
                    }
                };
                written += PString::write_string(name.to_string().as_bytes(), stream)?;
                Ok(written)
            }
            Property::UnsignedInt {
                size,
                default,
                name,
            } => {
                let mut key_type = PropType::UnsignedInt as u8;
                key_type += *size as u8 - 1;
                let mut written = match default {
                    None => stream.write_u8(key_type)?,
                    Some(d) => {
                        let mut written = 0;
                        written += stream.write_u8(key_type + 0b0000_1000)?;
                        written += stream.write_usized(*d, *size)?;
                        written
                    }
                };
                written += PString::write_string(name.to_string().as_bytes(), stream)?;
                Ok(written)
            }
            Property::SignedInt {
                size,
                default,
                name,
            } => {
                let mut key_type = PropType::SignedInt as u8;
                key_type += *size as u8 - 1;
                let mut written = match default {
                    None => stream.write_u8(key_type)?,
                    Some(d) => {
                        let mut written = 0;
                        written += stream.write_u8(key_type + 0b0000_1000)?;
                        written += stream.write_isized(*d, *size)?;
                        written
                    }
                };
                written += PString::write_string(name.to_string().as_bytes(), stream)?;
                Ok(written)
            }
            Property::Padding(size) => {
                let key_type = PropType::Padding as u8;
                stream.write_u8(key_type + (size - 1))
            }
        }
    }
}
