use super::super::PropertyName;
use super::ValueStore;
use crate::bases::Writable;
use crate::bases::*;
use std::cell::RefCell;
use std::rc::Rc;

pub enum Property<PN: PropertyName> {
    VariantId(String),
    Array {
        array_size_size: Option<ByteSize>,
        fixed_array_size: u8,
        deported_info: Option<(ByteSize, Rc<RefCell<ValueStore>>)>,
        name: PN,
    },
    ContentAddress {
        size: ByteSize,
        default: Option<u8>,
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
                array_size_size,
                fixed_array_size,
                deported_info,
                name,
            } => f
                .debug_struct("Array")
                .field("array_size_size", &array_size_size)
                .field("fixed_array_size", &fixed_array_size)
                .field("deported_info", &deported_info)
                .field("size", &self.size())
                .field("name", &name.to_string())
                .finish(),
            ContentAddress {
                size,
                default,
                name,
            } => f
                .debug_struct("ContentAddress")
                .field("size", &size)
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
                array_size_size,
                fixed_array_size,
                deported_info,
                name: _,
            } => {
                (match array_size_size {
                    None => 0,
                    Some(s) => *s as usize as u16,
                }) + *fixed_array_size as u16
                    + match deported_info {
                        None => 0,
                        Some((s, _)) => *s as usize as u16,
                    }
            }
            Property::ContentAddress {
                size,
                default,
                name: _,
            } => (if default.is_some() { 0 } else { 1 }) + *size as usize as u16,
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
                let mut written = stream.write_u8(0b1000_0000)?;
                written += PString::write_string(name.as_bytes(), stream)?;
                Ok(written)
            }
            Property::Array {
                array_size_size,
                fixed_array_size,
                deported_info,
                name,
            } => {
                let mut written = 0;
                let keytype = 0b0101_0000
                    + match array_size_size {
                        None => 0,
                        Some(s) => *s as usize as u8,
                    };
                written += stream.write_u8(keytype)?;
                let key_size = match deported_info {
                    None => 0,
                    Some((s, _)) => *s as usize as u8,
                } << 5;
                written += stream.write_u8(key_size + fixed_array_size)?;
                if let Some((_, store)) = deported_info {
                    written += store.borrow().get_idx().write(stream)?;
                }
                written += PString::write_string(name.to_string().as_bytes(), stream)?;
                Ok(written)
            }
            Property::ContentAddress {
                size,
                default,
                name,
            } => {
                let mut key_type = 0b0001_0000;
                key_type += *size as u8 - 1;
                let mut written = match default {
                    None => stream.write_u8(key_type + 0b0000_0100)?,
                    Some(d) => {
                        let mut written = 0;
                        written += stream.write_u8(key_type)?;
                        written += stream.write_u8(*d)?;
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
                let mut key_type = 0b0010_0000;
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
                let mut key_type = 0b0011_0000;
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
                let key_type = 0b0000_0000;
                stream.write_u8(key_type + (size - 1))
            }
        }
    }
}
