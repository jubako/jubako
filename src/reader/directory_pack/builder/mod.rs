mod property;

use super::layout::{Layout, Variant};
use super::raw_value::RawValue;
use super::LazyEntry;
use crate::bases::*;
use std::rc::Rc;

pub use self::property::*;

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

    pub fn new_from_variant(variant: &Variant) -> Self {
        let properties = variant.properties.iter().map(|p| p.into()).collect();
        Self { properties }
    }
}

pub struct AnyBuilder {
    variants: Vec<Rc<AnyVariantBuilder>>,
}

impl AnyBuilder {
    pub fn new_from_layout(layout: &Layout) -> Self {
        let variants = layout
            .variants
            .iter()
            .map(|v| Rc::new(AnyVariantBuilder::new_from_variant(v)))
            .collect();
        Self { variants }
    }
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
    use crate::reader::directory_pack::raw_layout::{RawProperty, RawPropertyKind};
    use crate::reader::directory_pack::{Array, EntryTrait};
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
        let builder = AnyBuilder::new_from_layout(&layout);

        {
            let content = vec![0x00, 0x00, 0x00, 0x01, 0x88, 0x99];

            let reader = Reader::new(content, End::None);
            let entry = builder.create_entry(0.into(), &reader).unwrap();

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
            let content = vec![0x01, 0x00, 0x00, 0x02, 0x66, 0x77];

            let reader = Reader::new(content, End::None);
            let entry = builder.create_entry(0.into(), &reader).unwrap();

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
        let builder = AnyBuilder::new_from_layout(&layout);

        {
            let content = vec![0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99];

            let reader = Reader::new(content, End::None);
            let entry = builder.create_entry(0.into(), &reader).unwrap();

            assert!(entry.get_variant_id() == 0.into());
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Array(Array::new(vec![0xFF, 0xEE, 0xDD, 0xCC], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::U16(0x8899));
        }

        {
            let content = vec![0x01, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99];

            let reader = Reader::new(content, End::None);
            let entry = builder.create_entry(0.into(), &reader).unwrap();

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
