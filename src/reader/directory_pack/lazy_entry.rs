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

    fn _get_value(&self, name: &str) -> Result<Option<RawValue>> {
        if self.properties.common.contains(name) {
            self.properties.common.create_value(name, &self.bytes)
        } else {
            self.properties
                .variant_part
                .as_ref()
                .and_then(|(id_property, variants, _)| {
                    let variant_id = match id_property.create(&self.bytes) {
                        Ok(v) => v,
                        Err(e) => return Some(Err(e)),
                    };
                    if variants.len() <= variant_id.into_usize() {
                        None
                    } else {
                        let variant = &variants[variant_id.into_usize()];
                        variant.create_value(name, &self.bytes).transpose()
                    }
                })
                .transpose()
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

    fn get_value(&self, name: &str) -> Result<Option<RawValue>> {
        self._get_value(name)
    }
}
