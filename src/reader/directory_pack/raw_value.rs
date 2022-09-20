use super::KeyStorage;
use crate::bases::*;
use crate::common::ContentAddress;

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

    pub fn pack_id(&self) -> Id<u8> {
        self.content_address.pack_id
    }

    pub fn content_id(&self) -> Idx<u32> {
        self.content_address.content_id
    }
}

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
    base: Vec<u8>,
    extend: Option<Extend>,
}

impl Array {
    pub fn new(base: Vec<u8>, extend: Option<Extend>) -> Self {
        Self { base, extend }
    }

    pub fn resolve_to_vec(&self, key_storage: &KeyStorage) -> Result<Vec<u8>> {
        Ok(match &self.extend {
            None => self.base.clone(),
            Some(e) => {
                let data = key_storage.get_data(e)?;
                [self.base.as_slice(), data.as_slice()].concat()
            }
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
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
