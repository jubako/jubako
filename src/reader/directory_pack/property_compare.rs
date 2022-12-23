use super::private::ValueStorageTrait;
use super::resolver::private::Resolver;
use super::{EntryTrait, LazyEntry, Value, ValueStorage};
use crate::bases::*;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub trait CompareTrait<E> {
    fn compare(&self, e: &E) -> Result<Ordering>;
}

pub(crate) mod private {
    use super::*;

    pub struct PropertyCompare<ValueStorage: ValueStorageTrait, Entry: EntryTrait> {
        resolver: Resolver<ValueStorage>,
        property_ids: Box<[PropertyIdx]>,
        values: Box<[Value]>,
        entry_type: PhantomData<Entry>,
    }

    impl<ValueStorage: ValueStorageTrait, Entry: EntryTrait> PropertyCompare<ValueStorage, Entry> {
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
                entry_type: PhantomData,
            }
        }
    }

    impl<ValueStorage: ValueStorageTrait, Entry: EntryTrait> CompareTrait<Entry>
        for PropertyCompare<ValueStorage, Entry>
    {
        fn compare(&self, e: &Entry) -> Result<Ordering> {
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

pub type PropertyCompare = private::PropertyCompare<ValueStorage, LazyEntry>;
