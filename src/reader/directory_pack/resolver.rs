use super::key_store::KeyStore;
use super::{Array, Content, DirectoryPack, Extend, RawValue};
use crate::bases::*;
use std::cell::OnceCell;
use std::rc::Rc;

#[derive(PartialEq, Eq)]
pub enum Value<'a> {
    Content(&'a Content),
    Unsigned(u64),
    Signed(i64),
    Array(Vec<u8>),
}

pub struct Resolver {
    directory: Rc<DirectoryPack>,
    stores: Vec<OnceCell<KeyStore>>,
}

impl Resolver {
    pub fn new(directory: Rc<DirectoryPack>) -> Self {
        let mut stores = Vec::new();
        stores.resize_with(directory.get_key_store_count().0 as usize, Default::default);
        Self { directory, stores }
    }

    fn get_key_store(&self, id: Idx<u8>) -> Result<&KeyStore> {
        self.stores[id.0 as usize].get_or_try_init(|| self._get_key_store(id))
    }

    fn _get_key_store(&self, id: Idx<u8>) -> Result<KeyStore> {
        self.directory.get_key_store(id)
    }

    fn get_data(&self, extend: &Extend) -> Result<Vec<u8>> {
        let key_store = self.get_key_store(extend.store_id)?;
        key_store.get_data(extend.key_id.into())
    }

    fn resolve_array_to_vec(&self, array: &Array) -> Result<Vec<u8>> {
        Ok(match &array.extend {
            None => array.base.clone(),
            Some(e) => {
                let data = self.get_data(e)?;
                [array.base.as_slice(), data.as_slice()].concat()
            }
        })
    }

    pub fn resolve<'a>(&self, raw: &'a RawValue) -> Result<Value<'a>> {
        Ok(match raw {
            RawValue::Content(c) => Value::Content(c),
            RawValue::U8(v) => Value::Unsigned(*v as u64),
            RawValue::U16(v) => Value::Unsigned(*v as u64),
            RawValue::U32(v) => Value::Unsigned(*v as u64),
            RawValue::U64(v) => Value::Unsigned(*v as u64),
            RawValue::I8(v) => Value::Signed(*v as i64),
            RawValue::I16(v) => Value::Signed(*v as i64),
            RawValue::I32(v) => Value::Signed(*v as i64),
            RawValue::I64(v) => Value::Signed(*v as i64),
            RawValue::Array(a) => Value::Array(self.resolve_array_to_vec(a)?),
        })
    }

    pub fn resolve_to_vec(&self, raw: &RawValue) -> Result<Vec<u8>> {
        if let RawValue::Array(a) = raw {
            self.resolve_array_to_vec(a)
        } else {
            panic!();
        }
    }

    pub fn resolve_to_content<'a>(&self, raw: &'a RawValue) -> &'a Content {
        if let RawValue::Content(c) = raw {
            c
        } else {
            panic!();
        }
    }

    pub fn resolve_to_unsigned(&self, raw: &RawValue) -> u64 {
        match raw {
            RawValue::U8(v) => *v as u64,
            RawValue::U16(v) => *v as u64,
            RawValue::U32(v) => *v as u64,
            RawValue::U64(v) => *v,
            _ => panic!(),
        }
    }

    pub fn resolve_to_signed(&self, raw: &RawValue) -> i64 {
        match raw {
            RawValue::I8(v) => *v as i64,
            RawValue::I16(v) => *v as i64,
            RawValue::I32(v) => *v as i64,
            RawValue::I64(v) => *v,
            _ => panic!(),
        }
    }
}
