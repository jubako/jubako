use super::key_store::KeyStore;
use super::DirectoryPack;
use super::Extend;
use crate::bases::*;
use std::cell::OnceCell;
use std::rc::Rc;

pub struct KeyStorage {
    directory: Rc<DirectoryPack>,
    stores: Vec<OnceCell<KeyStore>>,
}

impl KeyStorage {
    pub fn new(directory: Rc<DirectoryPack>) -> KeyStorage {
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

    pub fn get_data(&self, extend: &Extend) -> Result<Vec<u8>> {
        let key_store = self.get_key_store(extend.store_id)?;
        key_store.get_data(extend.key_id.into())
    }
}
