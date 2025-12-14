use super::super::layout;
use super::{ProcessedValue, PropertyName, StoreHandle, ValueStoreKind};
use crate::bases::*;
use std::cmp;

pub enum PropertySize<T> {
    Fixed(ByteSize),
    Auto(T),
}

impl<T> PropertySize<T>
where
    T: Ord + std::ops::Shr<Output = T> + From<u8> + Copy,
{
    fn process(&mut self, v: T) {
        match self {
            Self::Fixed(size) => {
                assert!(*size >= needed_bytes(v));
            }
            Self::Auto(max) => {
                *max = cmp::max(*max, v);
            }
        }
    }
}

impl<T: Default> Default for PropertySize<T> {
    fn default() -> Self {
        PropertySize::Auto(Default::default())
    }
}

impl<T> From<PropertySize<T>> for ByteSize
where
    T: Ord + std::ops::Shr<Output = T> + From<u8>,
{
    fn from(p: PropertySize<T>) -> ByteSize {
        match p {
            PropertySize::<T>::Fixed(size) => size,
            PropertySize::<T>::Auto(max) => needed_bytes(max),
        }
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

impl<T> From<ValueCounter<T>> for Option<T> {
    fn from(v: ValueCounter<T>) -> Option<T> {
        match v {
            ValueCounter::One(d) => Some(d),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct UnsignedInt {
    counter: ValueCounter<u64>,
    size: PropertySize<u64>,
}

impl UnsignedInt {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn absorb(&mut self, v: u64) -> ProcessedValue {
        self.counter.process(v);
        self.size.process(v);
        ProcessedValue::Unsigned(v)
    }

    fn finalize<PN: PropertyName>(self, name: PN) -> layout::Property<PN> {
        layout::Property::UnsignedInt {
            size: self.size.into(),
            default: self.counter.into(),
            name,
        }
    }
}

#[derive(Default)]
pub struct SignedInt {
    counter: ValueCounter<i64>,
    size: PropertySize<i64>,
}

impl SignedInt {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn absorb(&mut self, v: i64) -> ProcessedValue {
        self.counter.process(v);
        self.size.process(v);
        ProcessedValue::Signed(v)
    }

    fn finalize<PN: PropertyName>(self, name: PN) -> layout::Property<PN> {
        layout::Property::SignedInt {
            size: self.size.into(),
            default: self.counter.into(),
            name,
        }
    }
}

pub struct Array {
    max_array_size: PropertySize<usize>,
    fixed_array_len: usize,
    store_handle: StoreHandle,
}

impl Array {
    pub fn new(fixed_array_len: usize, store_handle: &StoreHandle) -> Self {
        Self {
            max_array_size: Default::default(),
            fixed_array_len,
            store_handle: store_handle.clone(),
        }
    }

    pub fn absorb(&mut self, data: SmallBytes) -> ProcessedValue {
        use super::super::value::{Array, ArrayS};
        let size = data.len();
        let (data, to_store) = data.split_at(cmp::min(self.fixed_array_len, data.len()));
        let value_id = self.store_handle.add_value(to_store);
        assert!(size <= 0x00FFFFFF_usize);
        self.max_array_size.process(size);
        match data.len() {
            0 => ProcessedValue::Array0(Box::new(ArrayS::<0> {
                size,
                value_id,
                data: data.try_into().unwrap(),
            })),
            1 => ProcessedValue::Array1(Box::new(ArrayS::<1> {
                size,
                value_id,
                data: data.try_into().unwrap(),
            })),
            2 => ProcessedValue::Array2(Box::new(ArrayS::<2> {
                size,
                value_id,
                data: data.try_into().unwrap(),
            })),
            _ => ProcessedValue::Array(Box::new(Array {
                size,
                data: data.into(),
                value_id,
            })),
        }
    }

    fn finalize<PN: PropertyName>(self, name: PN) -> layout::Property<PN> {
        let value_id_size = self.store_handle.key_size();
        layout::Property::Array {
            array_len_size: Some(self.max_array_size.into()),
            fixed_array_len: self.fixed_array_len as u8,
            deported_info: Some((value_id_size, self.store_handle.clone())),
            name,
        }
    }
}

pub struct IndirectArray {
    store_handle: StoreHandle,
}
impl IndirectArray {
    pub fn new(store_handle: StoreHandle) -> Self {
        Self { store_handle }
    }

    pub fn absorb(&mut self, data: SmallBytes) -> ProcessedValue {
        let value_id = self.store_handle.add_value(data);
        ProcessedValue::IndirectArray(Box::new(value_id))
    }
    fn finalize<PN: PropertyName>(self, name: PN) -> layout::Property<PN> {
        let value_id_size = self.store_handle.key_size();
        layout::Property::IndirectArray {
            value_id_size,
            store_handle: self.store_handle.clone(),
            name,
        }
    }
}

#[derive(Default)]
pub struct ContentAddress {
    pack_id_counter: ValueCounter<u16>,
    pack_id_size: PropertySize<u16>,
    content_id_size: PropertySize<u32>,
}
impl ContentAddress {
    pub fn absorb(&mut self, v: crate::common::ContentAddress) -> ProcessedValue {
        self.pack_id_counter.process(v.pack_id.into_u16());
        self.pack_id_size.process(v.pack_id.into_u16());
        self.content_id_size.process(v.content_id.into_u32());
        ProcessedValue::Content(v)
    }

    fn finalize<PN: PropertyName>(self, name: PN) -> layout::Property<PN> {
        layout::Property::ContentAddress {
            content_id_size: self.content_id_size.into(),
            pack_id_size: self.pack_id_size.into(),
            default: self.pack_id_counter.into(),
            name,
        }
    }
}
pub enum Property<PN: PropertyName> {
    UnsignedInt(UnsignedInt, PN),
    SignedInt(SignedInt, PN),
    Array(Array, PN),
    IndirectArray(IndirectArray, PN),
    ContentAddress(ContentAddress, PN),
    Padding(/*size*/ u8),
}

impl<PN: PropertyName> std::fmt::Debug for Property<PN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsignedInt(UnsignedInt { counter, size }, name) => f
                .debug_struct("UnsignedInt")
                .field("counter", &counter)
                .field("size", &size)
                .field("name", &name.as_str())
                .finish(),
            Self::SignedInt(SignedInt { counter, size }, name) => f
                .debug_struct("SignedInt")
                .field("counter", &counter)
                .field("size", &size)
                .field("name", &name.as_str())
                .finish(),
            Self::Array(
                Array {
                    max_array_size,
                    fixed_array_len,
                    store_handle,
                },
                name,
            ) => f
                .debug_struct("Array")
                .field("max_array_size", &max_array_size)
                .field("fixed_array_len", &fixed_array_len)
                .field("store_idx", &store_handle.get_idx())
                .field("key_size", &store_handle.key_size())
                .field("name", &name.as_str())
                .finish(),
            Self::IndirectArray(IndirectArray { store_handle }, name) => f
                .debug_struct("IndirectArray")
                .field("store_idx", &store_handle.get_idx())
                .field("key_size", &store_handle.key_size())
                .field("name", &name.as_str())
                .finish(),
            Self::ContentAddress(
                ContentAddress {
                    pack_id_counter,
                    pack_id_size,
                    content_id_size,
                },
                name,
            ) => f
                .debug_struct("ContentAddress")
                .field("pack_id_counter", &pack_id_counter)
                .field("pack_id_size", &pack_id_size)
                .field("content_id_size", &content_id_size)
                .field("name", &name.as_str())
                .finish(),
            Self::Padding(s) => f.debug_tuple("Padding").field(&s).finish(),
        }
    }
}

impl<PN: PropertyName> Property<PN> {
    pub fn new_uint(name: PN) -> Self {
        Property::UnsignedInt(Default::default(), name)
    }

    pub fn new_sint(name: PN) -> Self {
        Property::SignedInt(Default::default(), name)
    }

    pub fn new_array(fixed_array_len: usize, store_handle: StoreHandle, name: PN) -> Self {
        if fixed_array_len == 0 && store_handle.kind() == ValueStoreKind::Indexed {
            Property::IndirectArray(IndirectArray { store_handle }, name)
        } else {
            Property::Array(
                Array {
                    max_array_size: Default::default(),
                    fixed_array_len,
                    store_handle,
                },
                name,
            )
        }
    }

    pub fn new_content_address(name: PN) -> Self {
        Property::ContentAddress(Default::default(), name)
    }

    pub(crate) fn finalize(self) -> layout::Property<PN> {
        match self {
            Self::UnsignedInt(prop, name) => prop.finalize(name),
            Self::SignedInt(prop, name) => prop.finalize(name),
            Self::Array(prop, name) => prop.finalize(name),
            Self::IndirectArray(prop, name) => prop.finalize(name),
            Self::ContentAddress(prop, name) => prop.finalize(name),
            Self::Padding(size) => layout::Property::Padding(size),
        }
    }
}
