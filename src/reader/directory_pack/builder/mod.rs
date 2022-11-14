mod property;

use self::property::{AnyProperty, PropertyBuilderTrait};
use super::raw_value::RawValue;
use super::LazyEntry;
use crate::bases::*;
use std::rc::Rc;

pub trait BuilderTrait {
    type Entry;
    fn create_entry(&self, idx: EntryIdx, reader: &Reader) -> Result<Self::Entry>;
}

pub struct AnyVariantBuilder {
    properties: Vec<AnyProperty>,
}

impl AnyVariantBuilder {
    pub fn create_value(&self, idx: PropertyIdx, reader: &Reader) -> Result<RawValue> {
        self.properties[idx.into_usize()].create(reader)
    }
}

pub struct AnyBuilder {
    variants: Vec<Rc<AnyVariantBuilder>>,
}

impl BuilderTrait for AnyBuilder {
    type Entry = LazyEntry;

    fn create_entry(&self, _idx: EntryIdx, reader: &Reader) -> Result<LazyEntry> {
        let variant_id: VariantIdx = if self.variants.len() > 1 {
            reader.read_u8(Offset::zero())?
        } else {
            0
        }
        .into();
        let variant_builder = self.variants[variant_id.into_usize()];
        Ok(LazyEntry::new(
            variant_id,
            Rc::clone(&variant_builder),
            reader.create_sub_reader(Offset::zero(), End::None),
        ))
    }
}
