use super::entry_def::VariantDef;
use crate::bases::*;

/// A lazy entry
pub struct Entry<'a> {
    variant_id: u8,
    variant_def: &'a VariantDef,
    reader: Box<dyn Reader>,
}

impl<'a> Entry<'a> {
    pub fn new(variant_id: u8, variant_def: &'a VariantDef, reader: Box<dyn Reader>) -> Self {
        Self {
            variant_id,
            variant_def,
            reader,
        }
    }
}
