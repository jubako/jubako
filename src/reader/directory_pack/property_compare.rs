use super::builder::{AnyBuilder, BuilderTrait};
use super::finder::CompareTrait;
use super::private::ValueStorageTrait;
use super::resolver::private::Resolver;
use super::{EntryTrait, LazyEntry, Value, ValueStorage};
use crate::bases::*;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub(crate) mod private {
    use super::*;

    pub struct PropertyCompare<'builder, ValueStorage: ValueStorageTrait, Entry: EntryTrait> {
        resolver: Resolver<ValueStorage>,
        builder: &'builder AnyBuilder,
        property_ids: Box<[PropertyIdx]>,
        values: Box<[Value]>,
        entry_type: PhantomData<Entry>,
    }

    impl<'builder, ValueStorage: ValueStorageTrait, Entry: EntryTrait>
        PropertyCompare<'builder, ValueStorage, Entry>
    {
        pub fn new(
            resolver: Resolver<ValueStorage>,
            builder: &'builder AnyBuilder,
            property_ids: Vec<PropertyIdx>,
            values: Vec<Value>,
        ) -> Self {
            assert_eq!(property_ids.len(), values.len());
            Self {
                resolver,
                builder,
                property_ids: property_ids.into(),
                values: values.into(),
                entry_type: PhantomData,
            }
        }
    }

    impl<ValueStorage: ValueStorageTrait, Entry: EntryTrait> CompareTrait
        for PropertyCompare<'_, ValueStorage, Entry>
    {
        fn compare(&self, reader: &Reader) -> Result<Ordering> {
            let entry = self.builder.create_entry(EntryIdx::from(0), reader)?;
            for (property_id, value) in std::iter::zip(self.property_ids.iter(), self.values.iter())
            {
                let ordering = self
                    .resolver
                    .compare(&entry.get_value(*property_id)?, value)?;
                if ordering.is_ne() {
                    return Ok(ordering);
                }
            }
            Ok(Ordering::Equal)
        }
    }
}

pub type AnyPropertyCompare<'builder> = private::PropertyCompare<'builder, ValueStorage, LazyEntry>;
