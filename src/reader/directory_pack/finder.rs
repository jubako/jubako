use super::entry_store::EntryStoreTrait;
use super::property_compare::CompareTrait;
use super::EntryStore;
use crate::bases::*;
use std::rc::Rc;

mod private {
    use super::*;

    pub struct Finder<EntryStore: EntryStoreTrait> {
        store: Rc<EntryStore>,
        offset: EntryIdx,
        count: EntryCount,
    }

    impl<EntryStore: EntryStoreTrait> Finder<EntryStore> {
        pub fn new(store: Rc<EntryStore>, offset: EntryIdx, count: EntryCount) -> Self {
            Self {
                store,
                offset,
                count,
            }
        }

        fn _get_entry(&self, id: EntryIdx) -> Result<EntryStore::Entry> {
            self.store.get_entry(self.offset + id)
        }

        pub fn offset(&self) -> EntryIdx {
            self.offset
        }

        pub fn count(&self) -> EntryCount {
            self.count
        }

        pub fn get_store(&self) -> &Rc<EntryStore> {
            &self.store
        }

        pub fn get_entry(&self, id: EntryIdx) -> Result<EntryStore::Entry> {
            if id.is_valid(self.count) {
                self._get_entry(id)
            } else {
                Err("Invalid id".to_string().into())
            }
        }

        pub fn find<F>(&self, comparator: &F) -> Result<Option<EntryIdx>>
        where
            F: CompareTrait<EntryStore::Entry>,
        {
            for idx in self.count {
                let entry = self._get_entry(idx)?;
                let cmp = comparator.compare(&entry)?;
                if cmp.is_eq() {
                    return Ok(Some(idx));
                }
            }
            Ok(None)
        }
    }
}

pub type Finder = private::Finder<EntryStore>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::directory_pack::resolver::private::Resolver;
    use crate::reader::directory_pack::EntryTrait;
    use crate::reader::RawValue;
    use crate::reader::Value;
    use std::rc::Rc;

    mod mock {
        use super::*;
        use crate::reader::directory_pack::private::ValueStorageTrait;
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
            fn get_variant_id(&self) -> VariantIdx {
                0.into()
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

        pub type PropertyCompare =
            crate::reader::directory_pack::property_compare::private::PropertyCompare<
                ValueStorage,
                Entry,
            >;
    }

    #[test]
    fn test_finder() {
        let value_storage = Rc::new(mock::ValueStorage {});
        let resolver = Resolver::new(Rc::clone(&value_storage));
        let index_store = Rc::new(mock::EntryStore {});
        let finder = private::Finder::new(index_store, 0.into(), 10.into());
        for i in 0..10 {
            let entry = finder.get_entry(i.into()).unwrap();
            let value0 = entry.get_value(0.into()).unwrap();
            assert_eq!(resolver.resolve_to_unsigned(&value0), i as u64);
        }

        for i in 0..10 {
            let comparator =
                mock::PropertyCompare::new(resolver.clone(), 0.into(), Value::Unsigned(i));
            let idx = finder.find(&comparator).unwrap().unwrap();
            let entry = finder.get_entry(idx).unwrap();
            let value0 = entry.get_value(0.into()).unwrap();
            assert_eq!(resolver.resolve_to_unsigned(&value0), i as u64);
        }

        let comparator =
            mock::PropertyCompare::new(resolver.clone(), 0.into(), Value::Unsigned(10));
        let result = finder.find(&comparator).unwrap();
        assert_eq!(result, None);
    }
}
