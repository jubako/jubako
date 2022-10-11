use super::entry::EntryTrait;
use super::entry_def::VariantDef;
use super::raw_value::RawValue;
use crate::bases::*;
use std::rc::Rc;

/// A lazy entry
pub struct LazyEntry {
    variant_id: u8,
    variant_def: Rc<VariantDef>,
    reader: Box<dyn Reader>,
}

impl LazyEntry {
    pub fn new(variant_id: u8, variant_def: Rc<VariantDef>, reader: Box<dyn Reader>) -> Self {
        Self {
            variant_id,
            variant_def,
            reader,
        }
    }

    fn _get_value(&self, idx: Idx<u8>) -> Result<RawValue> {
        let key = &self.variant_def.keys[idx.0 as usize];
        key.create_value(self.reader.as_ref())
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
