use super::entry_store::EntryStoreTrait;
use super::private::ValueStorageTrait;
use super::resolver::private::Resolver;
use super::{EntryStore, EntryTrait, Value, ValueStorage};
use crate::bases::*;
use std::rc::Rc;

mod private {
    use super::*;
    pub struct Finder<K: ValueStorageTrait, IS: EntryStoreTrait> {
        store: Rc<IS>,
        offset: EntryIdx,
        count: EntryCount,
        resolver: Resolver<K>,
    }

    impl<K: ValueStorageTrait, IS: EntryStoreTrait> Finder<K, IS> {
        pub fn new(
            store: Rc<IS>,
            offset: EntryIdx,
            count: EntryCount,
            resolver: Resolver<K>,
        ) -> Self {
            Self {
                store,
                offset,
                count,
                resolver,
            }
        }

        fn _get_entry(&self, id: EntryIdx) -> Result<IS::Entry> {
            self.store.get_entry(self.offset + id)
        }

        pub fn offset(&self) -> EntryIdx {
            self.offset
        }

        pub fn count(&self) -> EntryCount {
            self.count
        }

        pub fn get_resolver(&self) -> &Resolver<K> {
            &self.resolver
        }

        pub fn get_store(&self) -> &Rc<IS> {
            &self.store
        }

        pub fn get_entry(&self, id: EntryIdx) -> Result<IS::Entry> {
            if id.is_valid(self.count) {
                self._get_entry(id)
            } else {
                Err("Invalid id".to_string().into())
            }
        }

        pub fn find(&self, property_index: PropertyIdx, value: Value) -> Result<Option<EntryIdx>> {
            for idx in self.count.into_iter() {
                let entry = self._get_entry(idx)?;
                let cmp = self
                    .resolver
                    .compare(&entry.get_value(property_index)?, &value)?;
                if cmp.is_eq() {
                    return Ok(Some(idx));
                }
            }
            Ok(None)
        }
    }
}

pub type Finder = private::Finder<ValueStorage, EntryStore>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::RawValue;
    use std::rc::Rc;

    mod mock {
        use super::*;
        use crate::reader::directory_pack::value_store::ValueStoreTrait;
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
            fn get_value(&self, idx: PropertyIdx) -> Result<RawValue> {
                Ok(match idx {
                    PropertyIdx(Idx(0)) => self.v.clone(),
                    _ => panic!(),
                })
            }
        }
        pub struct EntryStore {}
        impl EntryStoreTrait for EntryStore {
            type Entry = Entry;
            fn get_entry(&self, idx: EntryIdx) -> Result<Entry> {
                Ok(Entry::new(match idx {
                    EntryIdx(Idx(x)) if x < 10 => x as u16,
                    _ => panic!(),
                }))
            }
        }

        pub struct ValueStore {}
        impl ValueStoreTrait for ValueStore {
            fn get_data(&self, _id: ValueIdx) -> Result<&[u8]> {
                unreachable!()
            }
        }

        pub struct ValueStorage {}
        impl ValueStorageTrait for ValueStorage {
            type ValueStore = ValueStore;
            fn get_value_store(&self, _id: ValueStoreIdx) -> Result<&Rc<Self::ValueStore>> {
                unreachable!()
            }
        }
    }

    #[test]
    fn test_finder() {
        let value_storage = Rc::new(mock::ValueStorage {});
        let resolver = Resolver::new(Rc::clone(&value_storage));
        let index_store = Rc::new(mock::EntryStore {});
        let finder = private::Finder::new(index_store, 0.into(), 10.into(), resolver.clone());
        for i in 0..10 {
            let entry = finder.get_entry(i.into()).unwrap();
            let value0 = entry.get_value(0.into()).unwrap();
            assert_eq!(resolver.resolve_to_unsigned(&value0), i as u64);
        }

        for i in 0..10 {
            let idx = finder.find(0.into(), Value::Unsigned(i)).unwrap().unwrap();
            let entry = finder.get_entry(idx).unwrap();
            let value0 = entry.get_value(0.into()).unwrap();
            assert_eq!(resolver.resolve_to_unsigned(&value0), i as u64);
        }

        let result = finder.find(0.into(), Value::Unsigned(10)).unwrap();
        assert_eq!(result, None);
    }
}
