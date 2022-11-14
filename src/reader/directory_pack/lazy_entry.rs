use super::builder::AnyVariantBuilder;
use super::raw_value::RawValue;
use super::EntryTrait;
use crate::bases::*;
use std::rc::Rc;

/// A lazy entry
pub struct LazyEntry {
    variant_id: VariantIdx,
    builder: Rc<AnyVariantBuilder>,
    reader: Reader,
}

impl LazyEntry {
    pub fn new(variant_id: VariantIdx, builder: Rc<AnyVariantBuilder>, reader: Reader) -> Self {
        Self {
            variant_id,
            builder,
            reader,
        }
    }

    fn _get_value(&self, idx: PropertyIdx) -> Result<RawValue> {
        self.builder.create_value(idx, &self.reader)
    }
}

impl EntryTrait for LazyEntry {
    fn get_variant_id(&self) -> VariantIdx {
        self.variant_id
    }

    fn get_value(&self, idx: PropertyIdx) -> Result<RawValue> {
        self._get_value(idx)
    }
}
