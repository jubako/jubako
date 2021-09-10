use super::ContentAddress;
use crate::bases::*;

#[derive(Clone, PartialEq, Debug)]
pub struct Content {
    content_address: ContentAddress,
    base: Option<Box<Content>>,
}

impl Content {
    pub fn new(content_address: ContentAddress, base: Option<Content>) -> Self {
        Self {
            content_address,
            base: base.map(|c| Box::new(c)),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Extend {
    store_id: Idx<u8>,
    key_id: u64,
}

impl Extend {
    pub fn new(store_id: Idx<u8>, key_id: u64) -> Self {
        Self { store_id, key_id }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Array {
    base: Vec<u8>,
    extend: Option<Extend>,
}

impl Array {
    pub fn new(base: Vec<u8>, extend: Option<Extend>) -> Self {
        Self { base, extend }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
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
