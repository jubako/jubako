mod property;

use super::entry_store::EntryStore;
use super::layout::Variant;
use super::raw_value::RawValue;
use super::{AnyPropertyCompare, LazyEntry, Resolver, Value};
use crate::bases::*;
use std::rc::Rc;

pub use self::property::*;

pub trait BuilderTrait {
    type Entry;
    fn create_entry(&self, idx: EntryIdx) -> Result<Self::Entry>;
}

pub struct AnyVariantBuilder {
    properties: Vec<AnyProperty>,
}

impl AnyVariantBuilder {
    pub fn create_value(&self, idx: PropertyIdx, reader: &Reader) -> Result<RawValue> {
        self.properties[idx.into_usize()].create(reader)
    }

    pub fn new_from_variant(variant: &Variant) -> Self {
        let properties = variant.properties.iter().map(|p| p.into()).collect();
        Self { properties }
    }
}

pub struct AnyBuilder {
    variants: Vec<Rc<AnyVariantBuilder>>,
    store: Rc<EntryStore>,
}

impl AnyBuilder {
    pub fn new(store: Rc<EntryStore>) -> Self {
        let variants = store
            .layout()
            .variants
            .iter()
            .map(|v| Rc::new(AnyVariantBuilder::new_from_variant(v)))
            .collect();
        Self { variants, store }
    }

    pub fn new_property_compare(
        &self,
        resolver: Resolver,
        property_id: PropertyIdx,
        value: Value,
    ) -> AnyPropertyCompare {
        AnyPropertyCompare::new(resolver, self, vec![property_id], vec![value])
    }

    pub fn new_multiple_property_compare(
        &self,
        resolver: Resolver,
        property_ids: Vec<PropertyIdx>,
        values: Vec<Value>,
    ) -> AnyPropertyCompare {
        AnyPropertyCompare::new(resolver, self, property_ids, values)
    }
}

impl BuilderTrait for AnyBuilder {
    type Entry = LazyEntry;
    fn create_entry(&self, idx: EntryIdx) -> Result<LazyEntry> {
        let reader = self.store.get_entry_reader(idx);
        let variant_id: VariantIdx = if self.variants.len() > 1 {
            reader.read_u8(Offset::zero())?
        } else {
            0
        }
        .into();
        let variant_builder = &self.variants[variant_id.into_usize()];
        Ok(LazyEntry::new(
            variant_id,
            Rc::clone(variant_builder),
            reader.create_sub_reader(Offset::zero(), End::None),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ContentAddress;
    use crate::reader::directory_pack::entry_store::PlainStore;
    use crate::reader::directory_pack::raw_layout::{RawProperty, RawPropertyKind};
    use crate::reader::directory_pack::{Array, EntryTrait};
    use crate::reader::layout::Layout;
    use crate::reader::{Content, RawValue};

    #[test]
    fn create_entry() {
        let layout = Layout {
            variants: vec![Rc::new(
                Variant::new(vec![
                    RawProperty::new(RawPropertyKind::ContentAddress(0), 4),
                    RawProperty::new(RawPropertyKind::UnsignedInt, 2),
                ])
                .unwrap(),
            )],
            size: Size::new(6),
        };
        let content = vec![
            0x00, 0x00, 0x00, 0x01, 0x88, 0x99, 0x01, 0x00, 0x00, 0x02, 0x66, 0x77,
        ];
        let entry_reader = Reader::new(content, End::None);
        let store = Rc::new(EntryStore::Plain(PlainStore {
            layout,
            entry_reader,
        }));
        let builder = AnyBuilder::new(store);

        {
            let entry = builder.create_entry(0.into()).unwrap();

            assert!(entry.get_variant_id() == 0.into());
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Content(Content::new(
                        ContentAddress::new(0.into(), 1.into()),
                        None
                    ))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::U16(0x8899));
        }

        {
            let entry = builder.create_entry(1.into()).unwrap();

            assert!(entry.get_variant_id() == 0.into());
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Content(Content::new(
                        ContentAddress::new(1.into(), 2.into()),
                        None
                    ))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::U16(0x6677));
        }
    }

    #[test]
    fn create_entry_with_variant() {
        let layout = Layout {
            variants: vec![
                Rc::new(
                    Variant::new(vec![
                        RawProperty::new(RawPropertyKind::VariantId, 1),
                        RawProperty::new(RawPropertyKind::Array, 4),
                        RawProperty::new(RawPropertyKind::UnsignedInt, 2),
                    ])
                    .unwrap(),
                ),
                Rc::new(
                    Variant::new(vec![
                        RawProperty::new(RawPropertyKind::VariantId, 1),
                        RawProperty::new(RawPropertyKind::Array, 2),
                        RawProperty::new(RawPropertyKind::Padding, 1),
                        RawProperty::new(RawPropertyKind::SignedInt, 1),
                        RawProperty::new(RawPropertyKind::UnsignedInt, 2),
                    ])
                    .unwrap(),
                ),
            ],
            size: Size::new(7),
        };

        let content = vec![
            0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99, 0x01, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99,
        ];
        let entry_reader = Reader::new(content, End::None);
        let store = Rc::new(EntryStore::Plain(PlainStore {
            layout,
            entry_reader,
        }));
        let builder = AnyBuilder::new(store);

        {
            let entry = builder.create_entry(0.into()).unwrap();

            assert!(entry.get_variant_id() == 0.into());
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Array(Array::new(vec![0xFF, 0xEE, 0xDD, 0xCC], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::U16(0x8899));
        }

        {
            let entry = builder.create_entry(1.into()).unwrap();

            assert!(entry.get_variant_id() == 1.into());
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Array(Array::new(vec![0xFF, 0xEE], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::I8(-52));
            assert!(entry.get_value(2.into()).unwrap() == RawValue::U16(0x8899));
        }
    }
}
