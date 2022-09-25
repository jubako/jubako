use super::{IndexStore, LazyEntry, Resolver, Value};
use crate::bases::*;
use std::rc::Rc;

pub struct Finder {
    store: Rc<IndexStore>,
    offset: Idx<u32>,
    count: Count<u32>,
    resolver: Rc<Resolver>,
}

impl Finder {
    pub fn new(
        store: Rc<IndexStore>,
        offset: Idx<u32>,
        count: Count<u32>,
        resolver: Rc<Resolver>,
    ) -> Self {
        Self {
            store,
            offset,
            count,
            resolver,
        }
    }

    fn _get_entry(&self, id: Idx<u32>) -> Result<LazyEntry> {
        self.store.get_entry(self.offset + id)
    }

    pub fn count(&self) -> Count<u32> {
        self.count
    }

    pub fn get_resolver(&self) -> &Rc<Resolver> {
        &self.resolver
    }

    pub fn get_store(&self) -> &Rc<IndexStore> {
        &self.store
    }

    pub fn get_entry(&self, id: Idx<u32>) -> Result<LazyEntry> {
        if id.is_valid(self.count) {
            self._get_entry(id)
        } else {
            Err("Invalid id".to_string().into())
        }
    }

    pub fn find(&self, index_key: u8, value: Value) -> Result<Option<LazyEntry>> {
        for idx in 0..self.count.0 {
            let entry = self._get_entry(Idx(idx))?;
            let entry_value = self.resolver.resolve(entry.get_value(index_key.into())?)?;
            if entry_value == value {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }
}
