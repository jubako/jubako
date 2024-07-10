mod property;

use super::entry_store::EntryStore;
use super::layout::Properties as LProperties;
use super::layout::VariantPart;
use super::raw_value::RawValue;
use super::{LazyEntry, PropertyCompare};
use crate::bases::*;
use crate::common::Value;
use crate::reader::directory_pack::private::ValueStorageTrait;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub use self::property::*;

pub trait BuilderTrait {
    type Entry;
    fn create_entry(&self, idx: EntryIdx) -> Result<Self::Entry>;
}

pub struct AnyVariantBuilder {
    properties: HashMap<String, AnyProperty>,
}

impl AnyVariantBuilder {
    pub(super) fn contains(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }
    pub(super) fn create_value(&self, name: &str, parser: &impl RandomParser) -> Result<RawValue> {
        self.properties[name].create(parser)
    }

    pub(super) fn new<ValueStorage>(
        properties: &LProperties,
        value_storage: &ValueStorage,
    ) -> Result<Self>
    where
        ValueStorage: ValueStorageTrait,
    {
        let properties: Result<HashMap<String, _>> = properties
            .iter()
            .map(|(n, p)| match (p, value_storage).try_into() {
                Ok(p) => Ok((n.clone(), p)),
                Err(e) => Err(e),
            })
            .collect();
        Ok(Self {
            properties: properties?,
        })
    }

    pub(super) fn count(&self) -> u8 {
        self.properties.len() as u8
    }
}

pub(super) struct LazyEntryProperties {
    pub common: AnyVariantBuilder,
    pub variant_part: Option<(
        VariantIdProperty,
        Vec<AnyVariantBuilder>,
        HashMap<String, u8>,
    )>,
}

pub struct AnyBuilder {
    properties: Rc<LazyEntryProperties>,
    store: Arc<EntryStore>,
}

impl AnyBuilder {
    pub fn new<ValueStorage>(store: Arc<EntryStore>, value_storage: &ValueStorage) -> Result<Self>
    where
        ValueStorage: ValueStorageTrait,
    {
        let layout = store.layout();
        let common = AnyVariantBuilder::new(&layout.common, value_storage)?;
        let variant_part = match &layout.variant_part {
            None => None,
            Some(VariantPart {
                variant_id_offset,
                variants,
                names,
            }) => {
                let variants: Result<Vec<_>> = variants
                    .iter()
                    .map(|v| AnyVariantBuilder::new(v, value_storage))
                    .collect();
                let variant_id = VariantIdProperty::new(*variant_id_offset);
                Some((variant_id, variants?, names.clone()))
            }
        };
        let properties = Rc::new(LazyEntryProperties {
            common,
            variant_part,
        });
        Ok(Self { properties, store })
    }

    fn new_property_compare(&self, property_name: String, value: Value) -> PropertyCompare {
        PropertyCompare::new(self, vec![property_name], vec![value])
    }

    fn new_multiple_property_compare(
        &self,
        property_names: Vec<String>,
        values: Vec<Value>,
    ) -> PropertyCompare {
        PropertyCompare::new(self, property_names, values)
    }
}

impl BuilderTrait for AnyBuilder {
    type Entry = LazyEntry;
    fn create_entry(&self, idx: EntryIdx) -> Result<LazyEntry> {
        let reader = self.store.get_entry_reader(idx);
        Ok(LazyEntry::new(Rc::clone(&self.properties), reader.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ContentAddress;
    use crate::reader::directory_pack::entry_store::PlainStore;
    use crate::reader::directory_pack::raw_layout::{PropertyKind, RawProperty};
    use crate::reader::directory_pack::{Array, EntryTrait};
    use crate::reader::layout::{Layout, Properties};
    use crate::reader::RawValue;

    mod mock {
        use super::*;
        use crate::reader::directory_pack::private::ValueStorageTrait;
        use crate::reader::directory_pack::ValueStoreIdx;
        use crate::reader::directory_pack::ValueStoreTrait;
        use std::sync::Arc;

        #[derive(Debug)]
        pub struct ValueStore;
        impl ValueStoreTrait for ValueStore {
            fn get_data(&self, _id: ValueIdx, _size: Option<Size>) -> Result<&[u8]> {
                unreachable!();
            }
        }

        pub struct ValueStorage;
        impl ValueStorageTrait for ValueStorage {
            type ValueStore = ValueStore;
            fn get_value_store(&self, _idx: ValueStoreIdx) -> Result<Arc<Self::ValueStore>> {
                unreachable!();
            }
        }
    }

    #[test]
    fn create_entry() {
        let layout = Layout {
            common: Properties::new(
                0,
                vec![
                    RawProperty::new(
                        PropertyKind::ContentAddress {
                            pack_id_size: ByteSize::U1,
                            content_id_size: ByteSize::U3,
                            default_pack_id: None,
                        },
                        4,
                        Some("V0".to_string()),
                    ),
                    RawProperty::new(
                        PropertyKind::UnsignedInt {
                            int_size: ByteSize::U2,
                            default: None,
                        },
                        2,
                        Some("V11".to_string()),
                    ),
                ],
            )
            .unwrap(),
            variant_part: None,
            entry_count: EntryCount::from(2),
            is_entry_checked: false,
            entry_size: Size::new(6),
        };
        let entry_reader = Reader::from(vec![
            0x00, 0x00, 0x00, 0x01, 0x88, 0x99, 0x01, 0x00, 0x00, 0x02, 0x66, 0x77,
        ]);
        let store = Arc::new(EntryStore::Plain(PlainStore {
            layout,
            entry_reader,
        }));
        let value_storage = mock::ValueStorage {};
        let builder = AnyBuilder::new(store, &value_storage).unwrap();

        {
            let entry = builder.create_entry(0.into()).unwrap();

            assert!(entry.get_variant_id().unwrap().is_none());
            assert_eq!(
                entry.get_value("V0").unwrap(),
                RawValue::Content(ContentAddress::new(0.into(), 0x010000.into()))
            );
            assert!(entry.get_value("V11").unwrap() == RawValue::U16(0x9988));
        }

        {
            let entry = builder.create_entry(1.into()).unwrap();

            assert!(entry.get_variant_id().unwrap().is_none());
            assert_eq!(
                entry.get_value("V0").unwrap(),
                RawValue::Content(ContentAddress::new(1.into(), 0x020000.into()))
            );
            assert!(entry.get_value("V11").unwrap() == RawValue::U16(0x7766));
        }
    }

    #[test]
    fn create_entry_with_variant() {
        let layout = Layout {
            common: Properties::new(0, vec![]).unwrap(),
            variant_part: Some(VariantPart {
                variant_id_offset: Offset::new(0),
                variants: Box::new([
                    Properties::new(
                        1,
                        vec![
                            RawProperty::new(
                                PropertyKind::Array {
                                    array_len_size: Some(ByteSize::U1),
                                    fixed_array_len: 4,
                                    deported_info: None,
                                    default: None,
                                },
                                5,
                                Some("V0".to_string()),
                            ),
                            RawProperty::new(
                                PropertyKind::UnsignedInt {
                                    int_size: ByteSize::U2,
                                    default: None,
                                },
                                2,
                                Some("V1".to_string()),
                            ),
                        ],
                    )
                    .unwrap()
                    .into(),
                    Properties::new(
                        1,
                        vec![
                            RawProperty::new(
                                PropertyKind::Array {
                                    array_len_size: None,
                                    fixed_array_len: 2,
                                    deported_info: None,
                                    default: None,
                                },
                                2,
                                Some("V0".to_string()),
                            ),
                            RawProperty::new(PropertyKind::Padding, 2, Some("V1".to_string())),
                            RawProperty::new(
                                PropertyKind::SignedInt {
                                    int_size: ByteSize::U1,
                                    default: None,
                                },
                                1,
                                Some("V2".to_string()),
                            ),
                            RawProperty::new(
                                PropertyKind::UnsignedInt {
                                    int_size: ByteSize::U2,
                                    default: None,
                                },
                                2,
                                Some("V3".to_string()),
                            ),
                        ],
                    )
                    .unwrap()
                    .into(),
                ]),
                names: HashMap::from([
                    (String::from("Variant1"), 0),
                    (String::from("Variant2"), 1),
                ]),
            }),
            entry_count: EntryCount::from(2),
            is_entry_checked: false,
            entry_size: Size::new(8),
        };

        let entry_reader = Reader::from(vec![
            0x00, // Variant id entry 0
            0x04, 0xFF, 0xEE, 0xDD, 0xCC, // array entry 0
            0x99, 0x88, // uint entry 0
            0x01, // variant id entry 1
            0xFF, 0xEE, // array entry 1,
            0x00, 0x00, // Padding entry 1
            0xCC, // signed int entry 1
            0x99, 0x88, // uint entry 1
        ]);
        let store = Arc::new(EntryStore::Plain(PlainStore {
            layout,
            entry_reader,
        }));
        let value_storage = mock::ValueStorage {};
        let builder = AnyBuilder::new(store, &value_storage).unwrap();

        {
            let entry = builder.create_entry(0.into()).unwrap();

            assert_eq!(entry.get_variant_id().unwrap(), Some(0.into()));
            assert_eq!(
                entry.get_value("V0").unwrap(),
                RawValue::Array(Array::new(
                    Some(Size::new(4)),
                    BaseArray::new(&[0xFF, 0xEE, 0xDD, 0xCC]),
                    4,
                    None
                ))
            );
            assert_eq!(entry.get_value("V1").unwrap(), RawValue::U16(0x8899));
        }

        {
            let entry = builder.create_entry(1.into()).unwrap();

            assert_eq!(entry.get_variant_id().unwrap(), Some(1.into()));
            assert_eq!(
                entry.get_value("V0").unwrap(),
                RawValue::Array(Array::new(None, BaseArray::new(&[0xFF, 0xEE]), 2, None))
            );
            assert_eq!(entry.get_value("V2").unwrap(), RawValue::I8(-52));
            assert_eq!(entry.get_value("V3").unwrap(), RawValue::U16(0x8899));
        }
    }
}
