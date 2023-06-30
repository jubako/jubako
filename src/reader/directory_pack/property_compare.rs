use super::builder::{AnyBuilder, BuilderTrait};
use super::range::CompareTrait;
use super::{EntryTrait, Value};
use crate::bases::*;
use std::cmp::Ordering;

pub struct PropertyCompare<'builder> {
    builder: &'builder AnyBuilder,
    property_names: Box<[String]>,
    values: Box<[Value]>,
    ordered: bool,
}

impl<'builder> PropertyCompare<'builder> {
    pub fn new(
        builder: &'builder AnyBuilder,
        property_names: Vec<String>,
        values: Vec<Value>,
    ) -> Self {
        assert_eq!(property_names.len(), values.len());
        Self {
            builder,
            property_names: property_names.into(),
            values: values.into(),
            ordered: false,
        }
    }
}

impl CompareTrait for PropertyCompare<'_> {
    fn compare_entry(&self, idx: EntryIdx) -> Result<Ordering> {
        let entry = self.builder.create_entry(idx)?;
        for (name, value) in std::iter::zip(self.property_names.iter(), self.values.iter()) {
            let ordering = entry.get_value(name)?.partial_cmp(value).unwrap();
            if ordering.is_ne() {
                return Ok(ordering);
            }
        }
        Ok(Ordering::Equal)
    }

    fn ordered(&self) -> bool {
        self.ordered
    }
}
