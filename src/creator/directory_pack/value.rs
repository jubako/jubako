use super::ValueHandle;
use crate::bases::*;
use crate::common::ContentAddress;
use static_assertions as sa;
use std::cmp;

#[derive(Debug, PartialEq)]
pub struct Array {
    pub size: usize,
    pub data: Box<[u8]>,
    pub value_id: ValueHandle,
}

#[cfg(target_pointer_width = "64")]
sa::assert_eq_size!(Array, [u8; 40]);

#[cfg(target_pointer_width = "32")]
sa::assert_eq_size!(Array, [u8; 24]);

impl Array {
    fn cmp(&self, other: &Array) -> cmp::Ordering {
        match self.data.cmp(&other.data) {
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Greater => cmp::Ordering::Greater,
            cmp::Ordering::Equal => match self.value_id.get().cmp(&other.value_id.get()) {
                cmp::Ordering::Less => cmp::Ordering::Less,
                cmp::Ordering::Greater => cmp::Ordering::Greater,
                cmp::Ordering::Equal => self.size.cmp(&other.size),
            },
        }
    }
    fn cmp_array_s<const M: usize>(&self, other: &ArrayS<M>) -> cmp::Ordering {
        match self.data.as_ref().cmp(other.data.as_slice()) {
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Greater => cmp::Ordering::Greater,
            cmp::Ordering::Equal => match self.value_id.get().cmp(&other.value_id.get()) {
                cmp::Ordering::Less => cmp::Ordering::Less,
                cmp::Ordering::Greater => cmp::Ordering::Greater,
                cmp::Ordering::Equal => self.size.cmp(&other.size),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ArrayS<const N: usize> {
    pub data: [u8; N],
    pub value_id: ValueHandle,
    pub size: usize,
}
#[cfg(target_pointer_width = "64")]
sa::assert_eq_size!(ArrayS<0>, [u8; 24]);
#[cfg(target_pointer_width = "64")]
sa::assert_eq_size!(ArrayS<1>, [u8; 32]);
#[cfg(target_pointer_width = "64")]
sa::assert_eq_size!(ArrayS<2>, [u8; 32]);

#[cfg(target_pointer_width = "32")]
sa::assert_eq_size!(ArrayS<0>, [u8; 16]);
#[cfg(target_pointer_width = "32")]
sa::assert_eq_size!(ArrayS<1>, [u8; 20]);
#[cfg(target_pointer_width = "32")]
sa::assert_eq_size!(ArrayS<2>, [u8; 20]);

impl<const N: usize> ArrayS<N> {
    fn cmp<const M: usize>(&self, other: &ArrayS<M>) -> cmp::Ordering {
        match self.data.as_slice().cmp(other.data.as_slice()) {
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Greater => cmp::Ordering::Greater,
            cmp::Ordering::Equal => match self.value_id.get().cmp(&other.value_id.get()) {
                cmp::Ordering::Less => cmp::Ordering::Less,
                cmp::Ordering::Greater => cmp::Ordering::Greater,
                cmp::Ordering::Equal => self.size.cmp(&other.size),
            },
        }
    }
    fn cmp_array(&self, other: &Array) -> cmp::Ordering {
        match self.data.as_slice().cmp(&other.data) {
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Greater => cmp::Ordering::Greater,
            cmp::Ordering::Equal => match self.value_id.get().cmp(&other.value_id.get()) {
                cmp::Ordering::Less => cmp::Ordering::Less,
                cmp::Ordering::Greater => cmp::Ordering::Greater,
                cmp::Ordering::Equal => self.size.cmp(&other.size),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(u64),
    Signed(i64),
    UnsignedWord(Box<Word<u64>>),
    SignedWord(Box<Word<i64>>),
    IndirectArray(Box<ValueHandle>),
    Array0(Box<ArrayS<0>>),
    Array1(Box<ArrayS<1>>),
    Array2(Box<ArrayS<2>>),
    Array(Box<Array>),
}
#[cfg(target_pointer_width = "64")]
sa::assert_eq_size!(Value, [u8; 16]);

#[cfg(target_pointer_width = "32")]
sa::assert_eq_size!(Value, [u8; 12]);

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<cmp::Ordering> {
        match self {
            Value::Content(_) => None,
            Value::Unsigned(v) => match other {
                Value::Unsigned(o) => Some(v.cmp(o)),
                Value::UnsignedWord(o) => Some(v.cmp(&o.get())),
                _ => None,
            },
            Value::Signed(v) => match other {
                Value::Signed(o) => Some(v.cmp(o)),
                Value::SignedWord(o) => Some(v.cmp(&o.get())),
                _ => None,
            },
            Value::UnsignedWord(v) => match other {
                Value::Unsigned(o) => Some(v.get().cmp(o)),
                Value::UnsignedWord(o) => Some(v.get().cmp(&o.get())),
                _ => None,
            },
            Value::SignedWord(v) => match other {
                Value::Signed(o) => Some(v.get().cmp(o)),
                Value::SignedWord(o) => Some(v.get().cmp(&o.get())),
                _ => None,
            },
            Value::Array(v) => match other {
                Value::Array(other) => Some(v.cmp(other)),
                Value::Array0(other) => Some(v.cmp_array_s(other)),
                Value::Array1(other) => Some(v.cmp_array_s(other)),
                Value::Array2(other) => Some(v.cmp_array_s(other)),
                _ => None,
            },
            Value::Array0(v) => match other {
                Value::Array(other) => Some(v.cmp_array(other)),
                Value::Array0(other) => Some(v.cmp(other)),
                Value::Array1(other) => Some(v.cmp(other)),
                Value::Array2(other) => Some(v.cmp(other)),
                _ => None,
            },
            Value::Array1(v) => match other {
                Value::Array(other) => Some(v.cmp_array(other)),
                Value::Array0(other) => Some(v.cmp(other)),
                Value::Array1(other) => Some(v.cmp(other)),
                Value::Array2(other) => Some(v.cmp(other)),
                _ => None,
            },
            Value::Array2(v) => match other {
                Value::Array(other) => Some(v.cmp_array(other)),
                Value::Array0(other) => Some(v.cmp(other)),
                Value::Array1(other) => Some(v.cmp(other)),
                Value::Array2(other) => Some(v.cmp(other)),
                _ => None,
            },
            Value::IndirectArray(v) => match other {
                Value::IndirectArray(o) => Some(v.get().cmp(&o.get())),
                _ => None,
            },
        }
    }
}
