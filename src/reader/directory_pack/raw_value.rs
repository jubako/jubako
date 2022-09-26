use crate::bases::*;
use crate::common::Content;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Extend {
    pub(crate) store_id: Idx<u8>,
    pub(crate) key_id: u64,
}

impl Extend {
    pub fn new(store_id: Idx<u8>, key_id: u64) -> Self {
        Self { store_id, key_id }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Array {
    pub(super) base: Vec<u8>,
    pub(super) extend: Option<Extend>,
}

impl Array {
    pub fn new(base: Vec<u8>, extend: Option<Extend>) -> Self {
        Self { base, extend }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RawValue {
    Content(Content),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Array(Array),
}
