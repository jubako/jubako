use super::builder::{AnyBuilder, BuilderTrait};
use super::finder::CompareTrait;
use super::{EntryTrait, Value};
use crate::bases::*;
use std::cmp::Ordering;

pub struct PropertyCompare<'builder> {
    builder: &'builder AnyBuilder,
    property_ids: Box<[PropertyIdx]>,
    values: Box<[Value]>,
}

impl<'builder> PropertyCompare<'builder> {
    pub fn new(
        builder: &'builder AnyBuilder,
        property_ids: Vec<PropertyIdx>,
        values: Vec<Value>,
    ) -> Self {
        assert_eq!(property_ids.len(), values.len());
        Self {
            builder,
            property_ids: property_ids.into(),
            values: values.into(),
        }
    }
}

impl CompareTrait for PropertyCompare<'_> {
    fn compare_entry(&self, idx: EntryIdx) -> Result<Ordering> {
        let entry = self.builder.create_entry(idx)?;
        for (property_id, value) in std::iter::zip(self.property_ids.iter(), self.values.iter()) {
            let ordering = entry.get_value(*property_id)?.partial_cmp(value).unwrap();
            if ordering.is_ne() {
                return Ok(ordering);
            }
        }
        Ok(Ordering::Equal)
    }
}
