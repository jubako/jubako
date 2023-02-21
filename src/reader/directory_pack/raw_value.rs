use super::private::ValueStorageTrait;
use super::ValueStoreTrait;
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
    pub(super) size: Option<Size>,
    pub(super) base: BaseArray,
    pub(super) base_len: u8,
    pub(super) extend: Option<Extend>,
}

impl Array {
    pub fn new(size: Option<Size>, base: BaseArray, base_len: u8, extend: Option<Extend>) -> Self {
        Self {
            size,
            base,
            base_len,
            extend,
        }
    }
}

pub struct ArrayIter<'a, ValueStorage: ValueStorageTrait> {
    array: &'a Array,
    idx: usize,
    known_size: Option<usize>,
    value_store: Option<&'a ValueStorage::ValueStore>,
}

impl<'a, ValueStorage: ValueStorageTrait> ArrayIter<'a, ValueStorage> {
    pub fn new(array: &'a Array, value_store: Option<&'a ValueStorage::ValueStore>) -> Self {
        let known_size = array.size.map(|v| v.into_usize());
        Self {
            array,
            idx: 0,
            known_size,
            value_store,
        }
    }
}

impl<ValueStorage: ValueStorageTrait> Iterator for ArrayIter<'_, ValueStorage> {
    type Item = Result<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(s) = self.known_size {
            if self.idx >= s {
                return None;
            }
        }
        // As far as we know, we are under our known size, so we must return something.
        let base_len = self.array.base_len as usize;
        if self.idx < base_len {
            let ret = self.array.base.data[self.idx];
            self.idx += 1;
            Some(Ok(ret))
        } else if let Some(value_store) = self.value_store {
            let data = value_store.get_data(
                self.array.extend.as_ref().unwrap().value_id,
                self.array.size.map(|v| v - base_len.into()),
            );
            match data {
                Ok(data) => {
                    self.known_size = Some(base_len + data.len());
                    if self.idx - base_len < data.len() {
                        let ret = data[self.idx - base_len];
                        self.idx += 1;
                        Some(Ok(ret))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            }
        } else {
            self.known_size = Some(base_len);
            None
        }
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
