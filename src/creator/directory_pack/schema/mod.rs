mod properties;
mod property;

pub use properties::{CommonProperties, VariantProperties};
pub use property::{Array, ContentAddress, IndirectArray, Property, SignedInt, UnsignedInt};
use std::collections::HashMap;

use crate::creator::{directory_pack::ValueTransformer, BasicEntry};

use super::{layout, EntryTrait, PropertyName, StoreHandle, Value, ValueStoreKind, VariantName};
use properties::Properties;

#[derive(Debug)]
pub struct Schema<PN: PropertyName, VN: VariantName> {
    pub(crate) common: Properties<PN>,
    pub(crate) variants: Vec<(VN, Properties<PN>)>,
    pub(crate) sort_keys: Option<Vec<PN>>,
}

struct SortedIter<Item, Iter, CompFunc>
where
    Iter: Iterator<Item = Item>,
    CompFunc: Fn(&Item, &Item) -> bool,
{
    previous: Option<Item>,
    iter: Iter,
    compare: CompFunc,
}

impl<Item, Iter, CompFunc> SortedIter<Item, Iter, CompFunc>
where
    Iter: Iterator<Item = Item>,
    CompFunc: Fn(&Item, &Item) -> bool,
{
    fn new(iter: Iter, compare: CompFunc) -> Self {
        Self {
            previous: None,
            iter,
            compare,
        }
    }
}

impl<Item, Iter, CompFunc> Iterator for SortedIter<Item, Iter, CompFunc>
where
    Iter: Iterator<Item = Item>,
    CompFunc: Fn(&Item, &Item) -> bool,
{
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.previous.is_none() {
            self.previous = self.iter.next();
        }

        let next = self.iter.next();
        match (&self.previous, next) {
            (None, _) => None,
            (Some(_p), None) => self.previous.take(),
            (Some(p), Some(n)) => {
                assert!(
                    //p.compare(&self.keys, &n).is_le(),
                    (self.compare)(p, &n),
                    "Entry store is not sorted."
                );
                self.previous.replace(n)
            }
        }
    }
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

    pub fn build_entry(&mut self, entry: impl EntryTrait<PN, VN>) -> BasicEntry<VN> {
        let variant_name = entry.variant_name();
        let value_transformer = ValueTransformer::new(self, entry);
        BasicEntry {
            variant_name,
            values: value_transformer.collect::<Vec<_>>().into(),
        }
    }

    pub(crate) fn process_entries(
        &mut self,
        entries: Vec<impl EntryTrait<PN, VN>>,
    ) -> Vec<BasicEntry<VN>> {
        if let Some(keys) = &self.sort_keys.take() {
            let sorted_iter =
                SortedIter::new(entries.into_iter(), |p, n| p.compare(&keys, n).is_le());
            sorted_iter.map(|e| self.build_entry(e)).collect()
        } else {
            entries.into_iter().map(|e| self.build_entry(e)).collect()
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

#[cfg(test)]
mod tests {
    use rustest::test;

    use crate::creator::schema::SortedIter;

    #[test]
    fn sorted_iter_empty() {
        let iter = std::iter::empty::<u32>();
        let mut sorted_iter = SortedIter::new(iter, |_, _| true);
        assert_eq!(sorted_iter.next(), None);
    }

    #[test]
    fn sorted_iter_once() {
        let iter = std::iter::once(5);
        let mut sorted_iter = SortedIter::new(iter, |_, _| true);
        assert_eq!(sorted_iter.next(), Some(5));
        assert_eq!(sorted_iter.next(), None);
    }

    #[test]
    fn sorted_iter_sorted() {
        let iter = vec![5, 6, 8, 10].into_iter();
        let sorted_iter = SortedIter::new(iter, |a, b| a < b);
        assert_eq!(sorted_iter.collect::<Vec<_>>(), [5, 6, 8, 10]);
    }

    #[test]
    #[xfail]
    fn sorted_iter_not_sorted() {
        let iter = vec![5, 6, 8, 10, 9].into_iter();
        let sorted_iter = SortedIter::new(iter, |a, b| a < b);
        assert_eq!(sorted_iter.collect::<Vec<_>>(), [5, 6, 8, 10, 9]);
    }
}
