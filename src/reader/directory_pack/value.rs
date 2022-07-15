use super::ContentAddress;
use super::KeyStore;
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
            base: base.map(Box::new),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Extend {
    store_id: Idx<u8>,
    key_id: u64,
}

impl Extend {
    pub fn new(store_id: Idx<u8>, key_id: u64) -> Self {
        Self { store_id, key_id }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Array {
    base: Vec<u8>,
    extend: Option<Extend>,
}

impl Array {
    pub fn new(base: Vec<u8>, extend: Option<Extend>) -> Self {
        Self { base, extend }
    }

    pub fn resolve_to_vec(&self, key_store: &KeyStore) -> Result<Vec<u8>> {
        let extend = match &self.extend {
            None => Vec::new(),
            Some(e) => key_store.get_data(e.key_id.into())?,
        };
        Ok([self.base.as_slice(), extend.as_slice()].concat())
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
