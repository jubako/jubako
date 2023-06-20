mod properties;
mod property;

pub use properties::{CommonProperties, VariantProperties};
pub use property::Property;
use std::collections::HashMap;

use super::{layout, EntryTrait, Value, ValueStore};
use properties::Properties;

#[derive(Debug)]
pub struct Schema {
    pub common: Properties,
    pub variants: Vec<(String, Properties)>,
    pub sort_keys: Option<Vec<String>>,
}

impl Schema {
    pub fn new(
        common: CommonProperties,
        variants: Vec<(String, VariantProperties)>,
        sort_keys: Option<Vec<String>>,
    ) -> Self {
        Self {
            common,
            variants: variants
                .into_iter()
                .map(|(n, p)| (n, Properties::from(p)))
                .collect(),
            sort_keys,
        }
    }

    pub fn process(&mut self, entry: &dyn EntryTrait) {
        self.common.process(entry);
        if let Some(variant_name) = entry.variant_name() {
            for (n, p) in &mut self.variants {
                if n == variant_name {
                    p.process(entry);
                    break;
                }
            }
        }
    }

    pub fn finalize(&self) -> layout::Entry {
        let common_layout = self.common.finalize(None);
        let mut variants_layout = Vec::new();
        let mut variants_map = HashMap::new();
        for (name, variant) in &self.variants {
            variants_layout.push(variant.finalize(Some(name.to_string())));
            variants_map.insert(name.to_string(), (variants_layout.len() as u8 - 1).into());
        }
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
            variants_map,
            entry_size,
        }
    }
}
