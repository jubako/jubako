mod properties;
mod property;

pub use properties::{CommonProperties, VariantProperties};
pub use property::Property;

use super::{layout, EntryIter, EntryTrait, Value, ValueStore};
use crate::bases::*;
use properties::Properties;

#[derive(Debug)]
pub struct Schema {
    pub common: Properties,
    pub variants: Vec<Properties>,
    pub sort_keys: Option<Vec<PropertyIdx>>,
}

impl Schema {
    pub fn new(
        common: CommonProperties,
        variants: Vec<VariantProperties>,
        sort_keys: Option<Vec<PropertyIdx>>,
    ) -> Self {
        Self {
            common,
            variants: variants.into_iter().map(Properties::from).collect(),
            sort_keys,
        }
    }

    pub fn process(&mut self, entry: &dyn EntryTrait) {
        let mut iter = EntryIter::new(entry);
        self.common.process(&mut iter);
        if let Some(variant_id) = entry.variant_id() {
            self.variants[variant_id.into_usize()].process(&mut iter);
        }
    }

    pub fn finalize(&self) -> layout::Entry {
        let common_layout = self.common.finalize(false);
        let mut variants_layout: Vec<layout::Properties> =
            self.variants.iter().map(|v| v.finalize(true)).collect();
        let entry_size = if variants_layout.is_empty() {
            common_layout.entry_size()
        } else {
            let mut entry_size = common_layout.entry_size();
            let max_variant_size = variants_layout
                .iter()
                .map(|v| v.entry_size())
                .max()
                .unwrap();
            entry_size += max_variant_size;
            for variant in &mut variants_layout {
                variant.fill_to_size(max_variant_size);
            }
            entry_size
        };
        layout::Entry {
            common: common_layout,
            variants: variants_layout,
            entry_size,
        }
    }
}
