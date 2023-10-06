use crate::bases::*;
use crate::common::ContentAddress;
use std::cmp;

pub trait PropertyName: ToString + std::cmp::Eq + std::hash::Hash + Copy + Send + 'static {}
impl PropertyName for &'static str {}

pub trait VariantName: ToString + std::cmp::Eq + std::hash::Hash + Copy + Send {}
impl VariantName for &str {}

#[derive(Debug, PartialEq)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(Word<u64>),
    Signed(Word<i64>),
    Array {
        size: usize,
        data: Box<[u8]>,
        value_id: Bound<u64>,
    },
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<cmp::Ordering> {
        match self {
            Value::Content(_) => None,
            Value::Unsigned(v) => match other {
                Value::Unsigned(o) => Some(v.get().cmp(&o.get())),
                _ => None,
            },
            Value::Signed(v) => match other {
                Value::Signed(o) => Some(v.get().cmp(&o.get())),
                _ => None,
            },
            Value::Array {
                size,
                data,
                value_id: id,
            } => match other {
                Value::Array {
                    size: other_size,
                    data: other_data,
                    value_id: other_id,
                } => match data.cmp(other_data) {
                    cmp::Ordering::Less => Some(cmp::Ordering::Less),
                    cmp::Ordering::Greater => Some(cmp::Ordering::Greater),
                    cmp::Ordering::Equal => match id.get().cmp(&other_id.get()) {
                        cmp::Ordering::Less => Some(cmp::Ordering::Less),
                        cmp::Ordering::Greater => Some(cmp::Ordering::Greater),
                        cmp::Ordering::Equal => Some(size.cmp(other_size)),
                    },
                },
                _ => None,
            },
        }
    }
}
