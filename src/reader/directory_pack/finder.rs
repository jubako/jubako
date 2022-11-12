use super::builder::BuilderTrait;
use super::entry_store::{EntryStoreFront, EntryStoreTrait};
use super::property_compare::CompareTrait;
use super::schema::SchemaTrait;
use crate::bases::*;
use std::marker::PhantomData;
use std::rc::Rc;

mod private {
    use super::*;

    pub struct Finder<Schema: SchemaTrait, EntryStore>
    where
        EntryStore: EntryStoreTrait<Builder = Schema::Builder>,
    {
        store: Rc<EntryStore>,
        offset: EntryIdx,
        count: EntryCount,
        phantom_schema: PhantomData<Schema>,
    }

    impl<Schema: SchemaTrait, EntryStore> Finder<Schema, EntryStore>
    where
        EntryStore: EntryStoreTrait<Builder = Schema::Builder>,
    {
        pub fn new(store: Rc<EntryStore>, offset: EntryIdx, count: EntryCount) -> Self {
            Self {
                store,
                offset,
                count,
                phantom_schema: PhantomData,
            }
        }

        fn _get_entry(&self, id: EntryIdx) -> Result<<Schema::Builder as BuilderTrait>::Entry> {
            self.store.get_entry(self.offset + id)
        }

        pub fn offset(&self) -> EntryIdx {
            self.offset
        }

        pub fn count(&self) -> EntryCount {
            self.count
        }

        pub fn get_store(&self) -> Rc<EntryStore> {
            Rc::clone(&self.store)
        }

        pub fn get_entry(&self, id: EntryIdx) -> Result<<Schema::Builder as BuilderTrait>::Entry> {
            if id.is_valid(self.count) {
                self._get_entry(id)
            } else {
                Err("Invalid id".to_string().into())
            }
        }

        pub fn find<F>(&self, comparator: &F) -> Result<Option<EntryIdx>>
        where
            F: CompareTrait<<Schema::Builder as BuilderTrait>::Entry>,
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

pub type Finder<Schema> = private::Finder<Schema, EntryStoreFront<Schema>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::directory_pack::resolver::private::Resolver;
    use crate::reader::directory_pack::EntryTrait;
    use crate::reader::directory_pack::{builder, layout, schema};
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
        pub struct Builder {}
        impl builder::BuilderTrait for Builder {
            type Entry = Entry;
            fn create_entry(&self, idx: EntryIdx, _reader: &Reader) -> Result<Self::Entry> {
                Ok(Entry::new(idx.into_u16()))
            }
        }

        pub struct Schema {}

        impl schema::SchemaTrait for Schema {
            type Builder = Builder;
            fn check_layout(_layout: &layout::Layout) -> Result<Self::Builder> {
                Ok(Builder {})
            }
        }
        pub struct EntryStore {
            pub builder: Builder,
        }

        impl EntryStoreTrait for EntryStore {
            type Builder = Builder;
            fn get_entry(
                &self,
                idx: EntryIdx,
            ) -> Result<<Builder as builder::BuilderTrait>::Entry> {
                if idx.into_u32() >= 10 {
                    panic!()
                }
                self.builder
                    .create_entry(idx, &Reader::new(vec![], End::None))
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
        let builder = mock::Builder {};
        let entry_store = Rc::new(mock::EntryStore { builder });
        let finder: private::Finder<mock::Schema, mock::EntryStore> =
            private::Finder::new(entry_store, EntryIdx::from(0), EntryCount::from(10));

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
