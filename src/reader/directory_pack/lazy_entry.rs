use super::builder::{LazyEntryProperties, PropertyBuilderTrait};
use super::raw_value::RawValue;
use super::EntryTrait;
use crate::bases::*;
use std::rc::Rc;

/// A lazy entry
pub struct LazyEntry {
    properties: Rc<LazyEntryProperties>,
    reader: Reader,
}

impl LazyEntry {
    pub fn new(properties: Rc<LazyEntryProperties>, reader: Reader) -> Self {
        Self { properties, reader }
    }

    fn _get_value(&self, name: &str) -> Result<RawValue> {
        if self.properties.common.contains(name) {
            self.properties
                .common
                .create_value(name, &self.reader.as_sub_reader())
        } else {
            match &self.properties.variant_part {
                None => Err("Invalid key".to_string().into()),
                Some((id_property, variants, _)) => {
                    let sub_reader = self.reader.as_sub_reader();
                    let variant_id = id_property.create(&sub_reader)?;
                    let variant = &variants[variant_id.into_usize()];
                    variant.create_value(name, &sub_reader)
                }
            }
        }
    }
}

impl EntryTrait for LazyEntry {
    fn get_variant_id(&self) -> Result<Option<VariantIdx>> {
        match &self.properties.variant_part {
            None => Ok(None),
            Some((id_property, _, _)) => {
                Ok(Some(id_property.create(&self.reader.as_sub_reader())?))
            }
        }
    }

    fn get_value(&self, name: &str) -> Result<RawValue> {
        self._get_value(name)
    }
}
