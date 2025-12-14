use super::ValueHandle;
use crate::common::ContentAddress;
use std::cmp;

#[derive(Debug, PartialEq)]
pub struct Array {
    pub size: usize,
    pub data: Box<[u8]>,
    pub value_id: ValueHandle,
}

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

// Using a box here may seems odd but we try to reduce the size of ProcessedValue.
// Because we have a tag and because of alignement, ProcessedValue size is twice the
// size of the biggest value stored in the enum.
// By always keeping the this size to 8bytes, the enum size is 16 bytes.
//
// If we store a box to an Array the total size is 16 + 40 => 36
// If we store a u64, the size is 16
//
// If we don't store a box but directly the array, the total size of the enum is 48 bytes
// If we store a box to an Array the total size is 48
// If we store a u64, the size is 48
#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum ProcessedValue {
    Content(ContentAddress),
    Unsigned(u64),
    Signed(i64),
    IndirectArray(Box<ValueHandle>),
    Array0(Box<ArrayS<0>>),
    Array1(Box<ArrayS<1>>),
    Array2(Box<ArrayS<2>>),
    Array(Box<Array>),
}

impl PartialOrd for ProcessedValue {
    fn partial_cmp(&self, other: &ProcessedValue) -> Option<cmp::Ordering> {
        use ProcessedValue::*;
        match self {
            Content(_) => None,
            Unsigned(v) => match other {
                Unsigned(o) => Some(v.cmp(o)),
                _ => None,
            },
            Signed(v) => match other {
                Signed(o) => Some(v.cmp(o)),
                _ => None,
            },
            Array(v) => match other {
                Array(other) => Some(v.cmp(other)),
                Array0(other) => Some(v.cmp_array_s(other)),
                Array1(other) => Some(v.cmp_array_s(other)),
                Array2(other) => Some(v.cmp_array_s(other)),
                _ => None,
            },
            Array0(v) => match other {
                Array(other) => Some(v.cmp_array(other)),
                Array0(other) => Some(v.cmp(other)),
                Array1(other) => Some(v.cmp(other)),
                Array2(other) => Some(v.cmp(other)),
                _ => None,
            },
            Array1(v) => match other {
                Array(other) => Some(v.cmp_array(other)),
                Array0(other) => Some(v.cmp(other)),
                Array1(other) => Some(v.cmp(other)),
                Array2(other) => Some(v.cmp(other)),
                _ => None,
            },
            Array2(v) => match other {
                Array(other) => Some(v.cmp_array(other)),
                Array0(other) => Some(v.cmp(other)),
                Array1(other) => Some(v.cmp(other)),
                Array2(other) => Some(v.cmp(other)),
                _ => None,
            },
            IndirectArray(v) => match other {
                IndirectArray(o) => Some(v.get().cmp(&o.get())),
                _ => None,
            },
        }
    }
}
