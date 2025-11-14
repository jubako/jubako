mod properties;
mod property;

pub use properties::{CommonProperties, VariantProperties};
pub use property::{Array, ContentAddress, IndirectArray, Property, SignedInt, UnsignedInt};
use std::collections::HashMap;

use super::{layout, EntryTrait, PropertyName, StoreHandle, Value, ValueStoreKind, VariantName};
use properties::Properties;

#[derive(Debug)]
pub struct Schema<PN: PropertyName, VN: VariantName> {
    pub(crate) common: Properties<PN>,
    pub(crate) variants: Vec<(VN, Properties<PN>)>,
    pub(crate) sort_keys: Option<Vec<PN>>,
}

impl<PN: PropertyName, VN: VariantName> Schema<PN, VN> {
    pub fn new(
        common: CommonProperties<PN>,
        variants: Vec<(VN, VariantProperties<PN>)>,
        sort_keys: Option<Vec<PN>>,
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

    pub(crate) fn process(&mut self, entry: &dyn EntryTrait<PN, VN>) {
        self.common.process(entry);
        if let Some(variant_name) = entry.variant_name() {
            for (n, p) in &mut self.variants {
                if n == variant_name.as_ref() {
                    p.process(entry);
                    break;
                }
            }
        }
    }

    pub(crate) fn finalize(self) -> layout::Entry<PN, VN> {
        let common_layout = self.common.finalize(None);
        let mut variants_layout = Vec::new();
        let mut variants_map = HashMap::new();
        for (name, variant) in self.variants {
            variants_layout.push(variant.finalize(Some(name.as_str())));
            variants_map.insert(name, (variants_layout.len() as u8 - 1).into());
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
