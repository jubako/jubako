use super::entry_store::EntryStoreTrait;
use super::private::ValueStorageTrait;
use super::resolver::private::Resolver;
use super::{EntryStore, EntryTrait, Value, ValueStorage};
use crate::bases::*;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub trait CompareTrait<E> {
    fn compare(&self, e: &E) -> Result<Ordering>;
}

pub(crate) mod private {
    use super::*;

    pub struct PropertyCompare<ValueStorage: ValueStorageTrait, EntryStore: EntryStoreTrait> {
        resolver: Resolver<ValueStorage>,
        property_ids: Box<[PropertyIdx]>,
        values: Box<[Value]>,
        entry_store_type: PhantomData<EntryStore>,
    }

    impl<ValueStorage: ValueStorageTrait, EntryStore: EntryStoreTrait>
        PropertyCompare<ValueStorage, EntryStore>
    {
        pub fn new(
            resolver: Resolver<ValueStorage>,
            property_id: PropertyIdx,
            value: Value,
        ) -> Self {
            Self::new_multiple(resolver, vec![property_id], vec![value])
        }

        pub fn new_multiple(
            resolver: Resolver<ValueStorage>,
            property_ids: Vec<PropertyIdx>,
            values: Vec<Value>,
        ) -> Self {
            assert_eq!(property_ids.len(), values.len());
            Self {
                resolver,
                property_ids: property_ids.into(),
                values: values.into(),
                entry_store_type: PhantomData,
            }
        }
    }

    impl<ValueStorage: ValueStorageTrait, EntryStore: EntryStoreTrait>
        CompareTrait<EntryStore::Entry> for PropertyCompare<ValueStorage, EntryStore>
    {
        fn compare(&self, e: &EntryStore::Entry) -> Result<Ordering> {
            for (property_id, value) in std::iter::zip(self.property_ids.iter(), self.values.iter())
            {
                let ordering = self.resolver.compare(&e.get_value(*property_id)?, value)?;
                if ordering.is_ne() {
                    return Ok(ordering);
                }
            }
            Ok(Ordering::Equal)
        }
    }
}

pub type PropertyCompare = private::PropertyCompare<ValueStorage, EntryStore>;
