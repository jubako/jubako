use super::builder::{LazyEntryProperties, PropertyBuilderTrait};
use super::raw_value::RawValue;
use super::EntryTrait;
use crate::bases::*;
use crate::reader::ByteRegion;
use std::rc::Rc;

/// A lazy entry
pub struct LazyEntry {
    properties: Rc<LazyEntryProperties>,
    bytes: ByteRegion,
}

impl LazyEntry {
    pub(crate) fn new(properties: Rc<LazyEntryProperties>, bytes: ByteRegion) -> Self {
        Self { properties, bytes }
    }

    fn _get_value(&self, name: &str) -> Result<RawValue> {
        if self.properties.common.contains(name) {
            self.properties.common.create_value(name, &self.bytes)
        } else {
            match &self.properties.variant_part {
                None => Err("Invalid key".to_string().into()),
                Some((id_property, variants, _)) => {
                    let variant_id = id_property.create(&self.bytes)?;
                    let variant = &variants[variant_id.into_usize()];
                    variant.create_value(name, &self.bytes)
                }
            }
        }
    }
}

impl EntryTrait for LazyEntry {
    fn get_variant_id(&self) -> Result<Option<VariantIdx>> {
        match &self.properties.variant_part {
            None => Ok(None),
            Some((id_property, _, _)) => Ok(Some(id_property.create(&self.bytes)?)),
        }
    }

    fn get_value(&self, name: &str) -> Result<RawValue> {
        self._get_value(name)
    }
}
