use crate::bases::*;
use crate::common::ContentAddress;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Extend {
    pub(crate) store_id: ValueStoreIdx,
    pub(crate) value_id: ValueIdx,
}

impl Extend {
    pub fn new(store_id: ValueStoreIdx, value_id: ValueIdx) -> Self {
        Self { store_id, value_id }
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
    Content(ContentAddress),
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
