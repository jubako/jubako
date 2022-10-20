use super::entry::EntryTrait;
use super::entry_store::EntryStoreTrait;
use super::private::KeyStorageTrait;
use super::resolver::private::Resolver;
use super::{DirectoryPack, EntryStore, Value};
use crate::bases::*;
use std::rc::Rc;

mod private {
    use super::*;
    pub struct Finder<K: KeyStorageTrait, IS: EntryStoreTrait> {
        store: Rc<IS>,
        offset: Idx<u32>,
        count: Count<u32>,
        resolver: Rc<Resolver<K>>,
    }

    impl<K: KeyStorageTrait, IS: EntryStoreTrait> Finder<K, IS> {
        pub fn new(
            store: Rc<IS>,
            offset: Idx<u32>,
            count: Count<u32>,
            resolver: Rc<Resolver<K>>,
        ) -> Self {
            Self {
                store,
                offset,
                count,
                resolver,
            }
        }

        fn _get_entry(&self, id: Idx<u32>) -> Result<IS::Entry> {
            self.store.get_entry(self.offset + id)
        }

        pub fn offset(&self) -> Idx<u32> {
            self.offset
        }

        pub fn count(&self) -> Count<u32> {
            self.count
        }

        pub fn get_resolver(&self) -> &Rc<Resolver<K>> {
            &self.resolver
        }

        pub fn get_store(&self) -> &Rc<IS> {
            &self.store
        }

        pub fn get_entry(&self, id: Idx<u32>) -> Result<IS::Entry> {
            if id.is_valid(self.count) {
                self._get_entry(id)
            } else {
                Err("Invalid id".to_string().into())
            }
        }

        pub fn find(&self, property_index: u8, value: Value) -> Result<Option<Idx<u32>>> {
            for idx in 0..self.count.0 {
                let entry = self._get_entry(Idx(idx))?;
                let cmp = self
                    .resolver
                    .compare(&entry.get_value(property_index.into())?, &value)?;
                if cmp.is_eq() {
                    return Ok(Some(Idx(idx)));
                }
            }
            Ok(None)
        }
    }
}

pub type Finder = private::Finder<DirectoryPack, EntryStore>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::RawValue;

    mod mock {
        use super::*;
        use crate::reader::directory_pack::key_store::KeyStoreTrait;
        #[derive(PartialEq, Eq, Debug)]
        pub struct Entry {
            v: RawValue,
        }
        impl Entry {
            pub fn new(v: u16) -> Self {
                let v = RawValue::U16(v);
                Self { v }
            }
        }
        impl EntryTrait for Entry {
            fn get_variant_id(&self) -> u8 {
                0
            }
            fn get_value(&self, idx: Idx<u8>) -> Result<RawValue> {
                Ok(match idx {
                    Idx(0) => self.v.clone(),
                    _ => panic!(),
                })
            }
        }
        pub struct EntryStore {}
        impl EntryStoreTrait for EntryStore {
            type Entry = Entry;
            fn get_entry(&self, idx: Idx<u32>) -> Result<Entry> {
                Ok(Entry::new(match idx {
                    Idx(x) if x < 10 => x as u16,
                    _ => panic!(),
                }))
            }
        }

        pub struct KeyStore {}
        impl KeyStoreTrait for KeyStore {
            fn get_data(&self, _id: Idx<u64>) -> Result<Vec<u8>> {
                unreachable!()
            }
        }

        pub struct KeyStorage {}
        impl KeyStorageTrait for KeyStorage {
            type KeyStore = KeyStore;
            fn get_key_store_count(&self) -> Count<u8> {
                Count(0)
            }

            fn get_key_store(&self, _id: Idx<u8>) -> Result<Self::KeyStore> {
                unreachable!()
            }
        }
    }

    #[test]
    fn test_finder() {
        let key_storage = Rc::new(mock::KeyStorage {});
        let resolver = Rc::new(Resolver::new(key_storage));
        let index_store = Rc::new(mock::EntryStore {});
        let finder = private::Finder::new(index_store, Idx(0), Count(10), Rc::clone(&resolver));

        for i in 0..10 {
            let entry = finder.get_entry(Idx(i)).unwrap();
            let value0 = entry.get_value(Idx(0)).unwrap();
            assert_eq!(resolver.resolve_to_unsigned(&value0), i as u64);
        }

        for i in 0..10 {
            let idx = finder.find(0, Value::Unsigned(i)).unwrap().unwrap();
            let entry = finder.get_entry(idx).unwrap();
            let value0 = entry.get_value(Idx(0)).unwrap();
            assert_eq!(resolver.resolve_to_unsigned(&value0), i as u64);
        }

        let result = finder.find(0, Value::Unsigned(10)).unwrap();
        assert_eq!(result, None);
    }
}
