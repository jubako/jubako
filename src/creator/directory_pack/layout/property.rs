use super::ValueStore;
use crate::bases::Writable;
use crate::bases::*;
use std::cell::RefCell;
use std::rc::Rc;

pub enum Property {
    VariantId,
    Array {
        array_size_size: Option<ByteSize>,
        fixed_array_size: u8,
        deported_info: Option<(ByteSize, Rc<RefCell<ValueStore>>)>,
    },
    ContentAddress(ByteSize),
    UnsignedInt(ByteSize),
    Padding(/*size*/ u8),
}

impl std::fmt::Debug for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Property::*;
        match self {
            VariantId => f
                .debug_struct("VariantId")
                .field("size", &self.size())
                .finish(),
            Array {
                array_size_size,
                fixed_array_size,
                deported_info,
            } => f
                .debug_struct("Array")
                .field("array_size_size", &array_size_size)
                .field("fixed_array_size", &fixed_array_size)
                .field("deported_info", &deported_info)
                .field("size", &self.size())
                .finish(),
            ContentAddress(size) => f
                .debug_struct("ContentAddress")
                .field("size", &size)
                .field("size", &self.size())
                .finish(),
            UnsignedInt(size) => f
                .debug_struct("UnsignedInt")
                .field("size", &size)
                .field("size", &self.size())
                .finish(),
            Padding(_size) => f
                .debug_struct("Padding")
                .field("size", &self.size())
                .finish(),
        }
    }
}

impl Property {
    pub(crate) fn size(&self) -> u16 {
        match self {
            Property::VariantId => 1,
            Property::Array {
                array_size_size,
                fixed_array_size,
                deported_info,
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
            Property::ContentAddress(s) => *s as usize as u16 + 1,
            Property::UnsignedInt(size) => *size as u16,
            Property::Padding(size) => *size as u16,
        }
    }
}

impl Writable for Property {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        match self {
            Property::VariantId => stream.write_u8(0b1000_0000),
            Property::Array {
                array_size_size,
                fixed_array_size,
                deported_info,
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
                Ok(written)
            }
            Property::ContentAddress(s) => stream.write_u8(0b0001_0000 + (*s as usize as u8 - 1)),
            Property::UnsignedInt(size) => {
                let key_type = 0b0010_0000;
                stream.write_u8(key_type + (*size as u8 - 1))
            }
            Property::Padding(size) => {
                let key_type = 0b0000_0000;
                stream.write_u8(key_type + (size - 1))
            }
        }
    }
}
