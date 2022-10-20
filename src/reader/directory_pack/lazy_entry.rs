use super::entry::EntryTrait;
use super::layout;
use super::raw_value::RawValue;
use crate::bases::*;
use std::rc::Rc;

/// A lazy entry
pub struct LazyEntry {
    variant_id: u8,
    variant: Rc<layout::Variant>,
    reader: Box<dyn Reader>,
}

impl LazyEntry {
    pub fn new(variant_id: u8, variant: Rc<layout::Variant>, reader: Box<dyn Reader>) -> Self {
        Self {
            variant_id,
            variant,
            reader,
        }
    }

    fn _get_value(&self, idx: Idx<u8>) -> Result<RawValue> {
        let property = &self.variant.properties[idx.0 as usize];
        property.create_value(self.reader.as_ref())
    }
}

impl EntryTrait for LazyEntry {
    fn get_variant_id(&self) -> u8 {
        self.variant_id
    }

    fn get_value(&self, idx: Idx<u8>) -> Result<RawValue> {
        self._get_value(idx)
    }
}
