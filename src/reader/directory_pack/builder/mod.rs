mod property;

use super::entry_store::EntryStore;
use super::layout::Property as LProperty;
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
    pub fn create_value(&self, idx: PropertyIdx, reader: &SubReader) -> Result<RawValue> {
        self.properties[idx.into_usize()].create(reader)
    }

    pub fn new(properties: &[LProperty]) -> Self {
        let properties = properties.iter().map(|p| p.into()).collect();
        Self { properties }
    }

    pub fn count(&self) -> u8 {
        self.properties.len() as u8
    }
}

pub struct LazyEntryProperties {
    pub common: AnyVariantBuilder,
    pub variant_part: Option<(VariantIdProperty, Vec<AnyVariantBuilder>)>,
}

pub struct AnyBuilder {
    properties: Rc<LazyEntryProperties>,
    store: Rc<EntryStore>,
}

impl AnyBuilder {
    pub fn new(store: Rc<EntryStore>) -> Self {
        let layout = store.layout();
        let common = AnyVariantBuilder::new(&layout.common);
        let variant_part = match &layout.variant_part {
            None => None,
            Some((variant_id_offset, variants)) => {
                let variants = variants.iter().map(|v| AnyVariantBuilder::new(v)).collect();
                let variant_id = VariantIdProperty::new(*variant_id_offset);
                Some((variant_id, variants))
            }
        };
        let properties = Rc::new(LazyEntryProperties {
            common,
            variant_part,
        });
        Self { properties, store }
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
        let reader = self
            .store
            .get_entry_reader(idx)
            .create_sub_reader(Offset::zero(), End::None);
        Ok(LazyEntry::new(
            Rc::clone(&self.properties),
            reader.create_sub_reader(Offset::zero(), End::None).into(),
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
    use crate::reader::layout::{Layout, Properties};
    use crate::reader::RawValue;

    #[test]
    fn create_entry() {
        let layout = Layout {
            common: Properties::new(
                0,
                vec![
                    RawProperty::new(RawPropertyKind::ContentAddress, 4),
                    RawProperty::new(RawPropertyKind::UnsignedInt, 2),
                ],
            )
            .unwrap(),
            variant_part: None,
            size: Size::new(6),
        };
        let entry_reader = Reader::from(vec![
            0x00, 0x00, 0x00, 0x01, 0x88, 0x99, 0x01, 0x00, 0x00, 0x02, 0x66, 0x77,
        ]);
        let store = Rc::new(EntryStore::Plain(PlainStore {
            layout,
            entry_reader,
        }));
        let builder = AnyBuilder::new(store);

        {
            let entry = builder.create_entry(0.into()).unwrap();

            assert!(entry.get_variant_id().unwrap().is_none());
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Content(ContentAddress::new(0.into(), 1.into()),)
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::U16(0x8899));
        }

        {
            let entry = builder.create_entry(1.into()).unwrap();

            assert!(entry.get_variant_id().unwrap().is_none());
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Content(ContentAddress::new(1.into(), 2.into()),)
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::U16(0x6677));
        }
    }

    #[test]
    fn create_entry_with_variant() {
        let layout = Layout {
            common: Properties::new(0, vec![]).unwrap(),
            variant_part: Some((
                Offset::new(0),
                Box::new([
                    Properties::new(
                        1,
                        vec![
                            RawProperty::new(RawPropertyKind::Array, 4),
                            RawProperty::new(RawPropertyKind::UnsignedInt, 2),
                        ],
                    )
                    .unwrap()
                    .into(),
                    Properties::new(
                        1,
                        vec![
                            RawProperty::new(RawPropertyKind::Array, 2),
                            RawProperty::new(RawPropertyKind::Padding, 1),
                            RawProperty::new(RawPropertyKind::SignedInt, 1),
                            RawProperty::new(RawPropertyKind::UnsignedInt, 2),
                        ],
                    )
                    .unwrap()
                    .into(),
                ]),
            )),
            size: Size::new(7),
        };

        let entry_reader = Reader::from(vec![
            0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99, 0x01, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99,
        ]);
        let store = Rc::new(EntryStore::Plain(PlainStore {
            layout,
            entry_reader,
        }));
        let builder = AnyBuilder::new(store);

        {
            let entry = builder.create_entry(0.into()).unwrap();

            assert!(entry.get_variant_id().unwrap() == Some(0.into()));
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Array(Array::new(vec![0xFF, 0xEE, 0xDD, 0xCC], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::U16(0x8899));
        }

        {
            let entry = builder.create_entry(1.into()).unwrap();

            assert!(entry.get_variant_id().unwrap() == Some(1.into()));
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Array(Array::new(vec![0xFF, 0xEE], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::I8(-52));
            assert!(entry.get_value(2.into()).unwrap() == RawValue::U16(0x8899));
        }
    }
}
