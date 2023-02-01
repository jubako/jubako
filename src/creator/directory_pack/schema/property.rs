use super::super::layout;
use super::Value;
use super::ValueStore;
use crate::bases::*;
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;

#[derive(Debug)]
pub enum PropertySize<T> {
    Fixed(u8),
    Auto(T),
}

impl<T: Default> Default for PropertySize<T> {
    fn default() -> Self {
        PropertySize::Auto(Default::default())
    }
}

#[derive(Debug)]
pub enum Property {
    UnsignedInt(PropertySize<u64>),
    VLArray(
        /*flookup_size:*/ usize,
        /*store_handle:*/ Rc<RefCell<ValueStore>>,
    ),
    ContentAddress,
    Padding(/*size*/ u8),
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
                            assert!(*size <= needed_bytes(value.get()) as u8);
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
                PropertySize::Auto(max) => layout::Property::UnsignedInt(needed_bytes(*max) as u8),
            },
            Self::VLArray(flookup_size, store_handle) => {
                layout::Property::VLArray(*flookup_size, Rc::clone(store_handle))
            }
            Self::ContentAddress => layout::Property::ContentAddress,
            Self::Padding(size) => layout::Property::Padding(*size),
        }
    }
}
