use super::super::layout;
use super::Value;
use super::ValueStore;
use crate::bases::*;
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;

pub enum PropertySize<T> {
    Fixed(ByteSize),
    Auto(T),
}

impl<T: Default> Default for PropertySize<T> {
    fn default() -> Self {
        PropertySize::Auto(Default::default())
    }
}

impl<T> std::fmt::Debug for PropertySize<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(s) => f.write_str(&format!("FixedSize ({s:?})")),
            Self::Auto(m) => f.write_str(&format!("AutoSize (max:{m:?})")),
        }
    }
}

pub enum Property {
    UnsignedInt(PropertySize<u64>),
    VLArray(
        /*flookup_size:*/ usize,
        /*store_handle:*/ Rc<RefCell<ValueStore>>,
    ),
    ContentAddress,
    Padding(/*size*/ u8),
}

impl std::fmt::Debug for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsignedInt(s) => f.debug_tuple("UnsignedInt").field(&s).finish(),
            Self::VLArray(flookup_size, store_handle) => f
                .debug_struct("VLArray")
                .field("flookup_size", &flookup_size)
                .field("store_idx", &store_handle.borrow().get_idx())
                .field("key_size", &store_handle.borrow().key_size())
                .finish(),
            Self::ContentAddress => f.debug_tuple("ContentAddress").finish(),
            Self::Padding(s) => f.debug_tuple("Padding").field(&s).finish(),
        }
    }
}

impl Property {
    pub fn new_int() -> Self {
        Property::UnsignedInt(Default::default())
    }

    pub fn process<'a>(&mut self, values: &mut impl Iterator<Item = &'a Value>) {
        match self {
            Self::UnsignedInt(size) => {
                if let Value::Unsigned(value) = values.next().unwrap() {
                    match size {
                        PropertySize::Fixed(size) => {
                            assert!(*size <= needed_bytes(value.get()));
                        }
                        PropertySize::Auto(max) => {
                            *max = cmp::max(*max, value.get());
                        }
                    }
                } else {
                    panic!("Value type doesn't correspond to property");
                }
            }
            Self::Padding(_) => {
                panic!("Padding cannot process a value");
            }
            _ => {
                values.next();
            }
        }
    }

    pub fn finalize(&self) -> layout::Property {
        match self {
            Self::UnsignedInt(size) => match size {
                PropertySize::Fixed(size) => layout::Property::UnsignedInt(*size),
                PropertySize::Auto(max) => layout::Property::UnsignedInt(needed_bytes(*max)),
            },
            Self::VLArray(flookup_size, store_handle) => {
                layout::Property::VLArray(*flookup_size, Rc::clone(store_handle))
            }
            Self::ContentAddress => layout::Property::ContentAddress,
            Self::Padding(size) => layout::Property::Padding(*size),
        }
    }
}
