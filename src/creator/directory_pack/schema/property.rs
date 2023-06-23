use super::super::layout;
use super::{EntryTrait, PropertyName, Value, ValueStore};
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

#[derive(Default, Debug)]
pub enum ValueCounter<T> {
    #[default]
    None,
    One(T),
    Many,
}

impl<T> ValueCounter<T>
where
    T: PartialEq<T>,
{
    fn process(&mut self, v: T) {
        match self {
            Self::None => *self = Self::One(v),
            Self::One(d) => {
                if *d != v {
                    *self = Self::Many
                }
            }
            Self::Many => {}
        }
    }
}

pub enum Property<PN: PropertyName> {
    UnsignedInt {
        counter: ValueCounter<u64>,
        size: PropertySize<u64>,
        name: PN,
    },
    SignedInt {
        counter: ValueCounter<i64>,
        size: PropertySize<i64>,
        name: PN,
    },
    Array {
        max_array_size: PropertySize<usize>,
        fixed_array_size: usize,
        store_handle: Rc<RefCell<ValueStore>>,
        name: PN,
    },
    ContentAddress {
        pack_id_counter: ValueCounter<u8>,
        content_id_size: PropertySize<u32>,
        name: PN,
    },
    Padding(/*size*/ u8),
}

impl<PN: PropertyName> std::fmt::Debug for Property<PN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsignedInt {
                counter,
                size,
                name,
            } => f
                .debug_struct("UnsignedInt")
                .field("counter", &counter)
                .field("size", &size)
                .field("name", &name.to_string())
                .finish(),
            Self::SignedInt {
                counter,
                size,
                name,
            } => f
                .debug_struct("SignedInt")
                .field("counter", &counter)
                .field("size", &size)
                .field("name", &name.to_string())
                .finish(),
            Self::Array {
                max_array_size,
                fixed_array_size,
                store_handle,
                name,
            } => f
                .debug_struct("Array")
                .field("may_array_size", &max_array_size)
                .field("fixed_array_size", &fixed_array_size)
                .field("store_idx", &store_handle.borrow().get_idx())
                .field("key_size", &store_handle.borrow().key_size())
                .field("name", &name.to_string())
                .finish(),
            Self::ContentAddress {
                pack_id_counter,
                content_id_size,
                name,
            } => f
                .debug_struct("ContentAddress")
                .field("pack_id_counter", &pack_id_counter)
                .field("content_id_size", &content_id_size)
                .field("name", &name.to_string())
                .finish(),
            Self::Padding(s) => f.debug_tuple("Padding").field(&s).finish(),
        }
    }
}

impl<PN: PropertyName> Property<PN> {
    pub fn new_uint(name: PN) -> Self {
        Property::UnsignedInt {
            counter: Default::default(),
            size: Default::default(),
            name,
        }
    }

    pub fn new_sint(name: PN) -> Self {
        Property::SignedInt {
            counter: Default::default(),
            size: Default::default(),
            name,
        }
    }

    pub fn new_array(
        fixed_array_size: usize,
        store_handle: Rc<RefCell<ValueStore>>,
        name: PN,
    ) -> Self {
        Property::Array {
            max_array_size: Default::default(),
            fixed_array_size,
            store_handle,
            name,
        }
    }

    pub fn new_content_address(name: PN) -> Self {
        Property::ContentAddress {
            pack_id_counter: Default::default(),
            content_id_size: Default::default(),
            name,
        }
    }

    pub fn process(&mut self, entry: &dyn EntryTrait<PN>) {
        match self {
            Self::UnsignedInt {
                counter,
                size,
                name,
            } => {
                if let Value::Unsigned(value) = entry.value(name) {
                    counter.process(value.get());
                    match size {
                        PropertySize::Fixed(size) => {
                            assert!(*size >= needed_bytes(value.get()));
                        }
                        PropertySize::Auto(max) => {
                            *max = cmp::max(*max, value.get());
                        }
                    }
                } else {
                    panic!("Value type doesn't correspond to property");
                }
            }
            Self::SignedInt {
                counter,
                size,
                name,
            } => {
                if let Value::Signed(value) = entry.value(name) {
                    counter.process(value.get());
                    match size {
                        PropertySize::Fixed(size) => {
                            assert!(*size >= needed_bytes(value.get()));
                        }
                        PropertySize::Auto(max) => {
                            *max = cmp::max(*max, value.get());
                        }
                    }
                } else {
                    panic!("Value type doesn't correspond to property");
                }
            }
            Self::ContentAddress {
                pack_id_counter,
                content_id_size,
                name,
            } => {
                if let Value::Content(c) = entry.value(name) {
                    pack_id_counter.process(c.pack_id.into_u8());
                    match content_id_size {
                        PropertySize::Fixed(size) => {
                            assert!(*size >= needed_bytes(c.content_id.into_u32()));
                        }
                        PropertySize::Auto(max) => {
                            *max = cmp::max(*max, c.content_id.into_u32());
                        }
                    }
                } else {
                    panic!("Value type doesn't correspond to property");
                }
            }
            Self::Array {
                max_array_size,
                fixed_array_size: _,
                store_handle: _,
                name,
            } => {
                if let Value::Array {
                    size,
                    data: _,
                    value_id: _,
                } = entry.value(name)
                {
                    match max_array_size {
                        PropertySize::Fixed(fixed_size) => {
                            assert!(*fixed_size >= needed_bytes(*size));
                        }
                        PropertySize::Auto(max) => {
                            *max = cmp::max(*max, *size);
                        }
                    }
                } else {
                    panic!("Value type doesn't correspond to property");
                }
            }
            Self::Padding(_) => {
                panic!("Padding cannot process a value");
            }
        }
    }

    pub fn finalize(&self) -> layout::Property<PN> {
        match self {
            Self::UnsignedInt {
                counter,
                size,
                name,
            } => {
                let size = match size {
                    PropertySize::Fixed(size) => *size,
                    PropertySize::Auto(max) => needed_bytes(*max),
                };
                match counter {
                    ValueCounter::One(d) => layout::Property::UnsignedInt {
                        size,
                        default: Some(*d),
                        name: *name,
                    },
                    _ => layout::Property::UnsignedInt {
                        size,
                        default: None,
                        name: *name,
                    },
                }
            }
            Self::SignedInt {
                counter,
                size,
                name,
            } => {
                let size = match size {
                    PropertySize::Fixed(size) => *size,
                    PropertySize::Auto(max) => needed_bytes(*max),
                };
                match counter {
                    ValueCounter::One(d) => layout::Property::SignedInt {
                        size,
                        default: Some(*d),
                        name: *name,
                    },
                    _ => layout::Property::SignedInt {
                        size,
                        default: None,
                        name: *name,
                    },
                }
            }
            Self::Array {
                max_array_size,
                fixed_array_size,
                store_handle,
                name,
            } => {
                let value_id_size = store_handle.borrow().key_size();
                layout::Property::Array {
                    array_size_size: Some(match max_array_size {
                        PropertySize::Fixed(size) => *size,
                        PropertySize::Auto(max) => needed_bytes(*max),
                    }),
                    fixed_array_size: *fixed_array_size as u8,
                    deported_info: Some((value_id_size, Rc::clone(store_handle))),
                    name: *name,
                }
            }
            Self::ContentAddress {
                pack_id_counter,
                content_id_size,
                name,
            } => {
                let default = match pack_id_counter {
                    ValueCounter::One(d) => Some(*d),
                    _ => None,
                };
                let size = match content_id_size {
                    PropertySize::Fixed(size) => *size,
                    PropertySize::Auto(max) => needed_bytes(*max),
                };
                layout::Property::ContentAddress {
                    size,
                    default,
                    name: *name,
                }
            }
            Self::Padding(size) => layout::Property::Padding(*size),
        }
    }
}
