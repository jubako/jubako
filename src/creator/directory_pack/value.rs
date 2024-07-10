use super::ValueHandle;
use crate::bases::*;
use crate::common::ContentAddress;
use static_assertions as sa;
use std::cmp;

trait PropertyName: ToString + std::cmp::Eq + std::hash::Hash + Copy + Send + 'static {}
impl PropertyName for &'static str {}

trait VariantName: ToString + std::cmp::Eq + std::hash::Hash + Copy + Send {}
impl VariantName for &str {}

#[derive(Debug, PartialEq)]
pub struct Array {
    pub size: usize,
    pub data: Box<[u8]>,
    pub value_id: ValueHandle,
}
sa::assert_eq_size!(Array, [u8; 40]);

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
sa::assert_eq_size!(ArrayS<0>, [u8; 24]);
sa::assert_eq_size!(ArrayS<1>, [u8; 32]);
sa::assert_eq_size!(ArrayS<2>, [u8; 32]);

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
sa::assert_eq_size!(Value, [u8; 16]);

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
