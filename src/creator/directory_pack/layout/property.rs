use super::ValueStore;
use crate::bases::Writable;
use crate::bases::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub enum Property {
    VariantId,
    VLArray(
        /*flookup_size:*/ usize,
        /*store_handle:*/ Rc<RefCell<ValueStore>>,
    ),
    ContentAddress,
    UnsignedInt(ByteSize),
    Padding(/*size*/ u8),
}

impl Property {
    pub(crate) fn size(&self) -> u16 {
        match self {
            Property::VariantId => 1,
            Property::VLArray(flookup_size, store_handle) => {
                (*flookup_size as u16) + store_handle.borrow().key_size() as u16
            }
            Property::ContentAddress => 4,
            Property::UnsignedInt(size) => *size as u16,
            Property::Padding(size) => *size as u16,
        }
    }

    pub(crate) fn key_count(&self) -> u8 {
        match self {
            Property::VLArray(flookup_size, _) => {
                if *flookup_size > 0 {
                    2
                } else {
                    1
                }
            }
            Property::VariantId => 1,
            Property::ContentAddress => 1,
            Property::UnsignedInt(_) => 1,
            Property::Padding(_) => 1,
        }
    }
}

impl Writable for Property {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        match self {
            Property::VariantId => stream.write_u8(0b1000_0000),
            Property::VLArray(flookup_size, store_handle) => {
                let mut flookup_size = *flookup_size;
                let keytype = if flookup_size > 0 {
                    0b0111_0000
                } else {
                    0b0110_0000
                };
                let key_size = store_handle.borrow().key_size() as u8 - 1;
                let mut written = 0;
                written += stream.write_u8(keytype + key_size)?;
                written += store_handle.borrow().get_idx().write(stream)?;
                if flookup_size > 0 {
                    let keytype: u8 = 0b0100_0000;
                    if flookup_size <= 8 {
                        written += stream.write_u8(keytype + (flookup_size - 1) as u8)?;
                    } else if flookup_size <= 2056 {
                        flookup_size -= 9;
                        written += stream
                            .write_u8(keytype + ((flookup_size >> 8) & 0x03) as u8 + 0b1000)?;
                        written += stream.write_u8(flookup_size as u8)?;
                    } else {
                        panic!()
                    }
                }
                Ok(written)
            }
            Property::ContentAddress => stream.write_u8(0b0001_0000),
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
