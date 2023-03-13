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

    fn _get_value(&self, idx: PropertyIdx) -> Result<RawValue> {
        let common_len = self.properties.common.count();
        if idx.into_u8() < common_len {
            self.properties
                .common
                .create_value(idx, &self.reader.as_sub_reader())
        } else {
            match &self.properties.variant_part {
                None => Err("Invalid key".to_string().into()),
                Some((id_property, variants)) => {
                    let sub_reader = self.reader.as_sub_reader();
                    let variant_id = id_property.create(&sub_reader)?;
                    let variant = &variants[variant_id.into_usize()];
                    variant.create_value(idx - common_len, &sub_reader)
                }
            }
        }
    }
}

impl EntryTrait for LazyEntry {
    fn get_variant_id(&self) -> Result<Option<VariantIdx>> {
        match &self.properties.variant_part {
            None => Ok(None),
            Some((id_property, _)) => Ok(Some(id_property.create(&self.reader.as_sub_reader())?)),
        }
    }

    fn get_value(&self, idx: PropertyIdx) -> Result<RawValue> {
        self._get_value(idx)
    }
}
