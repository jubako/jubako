use super::entry_def::VariantDef;
use super::value::Value;
use crate::bases::*;
use std::lazy::OnceCell;

/// A lazy entry
pub struct Entry<'a> {
    variant_id: u8,
    variant_def: &'a VariantDef,
    values: Vec<OnceCell<Value>>,
    reader: Box<dyn Reader>,
}

impl<'a> Entry<'a> {
    pub fn new(variant_id: u8, variant_def: &'a VariantDef, reader: Box<dyn Reader>) -> Self {
        let mut values = Vec::new();
        values.resize_with(variant_def.keys.len(), Default::default);
        Self {
            variant_id,
            variant_def,
            values,
            reader,
        }
    }

    pub fn get_variant_id(&self) -> u8 {
        self.variant_id
    }

    pub fn get_value(&self, idx: Idx<u8>) -> Result<&Value> {
        self.values[idx.0 as usize].get_or_try_init(|| self._get_value(idx))
    }

    fn _get_value(&self, idx: Idx<u8>) -> Result<Value> {
        let key = &self.variant_def.keys[idx.0 as usize];
        key.create_value(self.reader.as_ref())
    }
}
