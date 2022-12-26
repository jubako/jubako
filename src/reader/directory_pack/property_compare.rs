use super::builder::{AnyBuilder, BuilderTrait};
use super::finder::CompareTrait;
use super::private::ValueStorageTrait;
use super::resolver::private::Resolver;
use super::schema::AnySchema;
use super::{EntryTrait, Value, ValueStorage};
use crate::bases::*;
use std::cmp::Ordering;

pub(crate) mod private {
    use super::*;

    pub struct PropertyCompare<'builder, ValueStorage: ValueStorageTrait> {
        resolver: Resolver<ValueStorage>,
        builder: &'builder AnyBuilder,
        property_ids: Box<[PropertyIdx]>,
        values: Box<[Value]>,
    }

    impl<'builder, ValueStorage: ValueStorageTrait> PropertyCompare<'builder, ValueStorage> {
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
            }
        }
    }

    impl<ValueStorage: ValueStorageTrait> CompareTrait<AnySchema>
        for PropertyCompare<'_, ValueStorage>
    {
        fn compare_entry(&self, idx: EntryIdx) -> Result<Ordering> {
            let entry = self.builder.create_entry(idx)?;
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

pub type AnyPropertyCompare<'builder> = private::PropertyCompare<'builder, ValueStorage>;
