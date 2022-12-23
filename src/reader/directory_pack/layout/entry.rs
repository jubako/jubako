use super::raw_property::{RawProperty, RawPropertyKind};
use super::variant::Variant;
use super::LazyEntry;
use crate::bases::*;
use std::cmp::Ordering;
use std::rc::Rc;

#[derive(Debug)]
pub struct Entry {
    pub variants: Vec<Rc<Variant>>,
    pub size: Size,
}

impl Producable for Entry {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Entry> {
        let entry_size = stream.read_u16()? as usize;
        let variant_count: VariantCount = Count::<u8>::produce(stream)?.into();
        let raw_property_count: PropertyCount = Count::<u8>::produce(stream)?.into();
        let mut variants = Vec::new();
        let mut entry_def = Vec::new();
        let mut current_size = 0;
        for _ in raw_property_count {
            let raw_property = RawProperty::produce(stream)?;
            if raw_property.kind == RawPropertyKind::VariantId && !entry_def.is_empty() {
                return Err(format_error!(
                    "VariantId cannot appear in the middle of a entry.",
                    stream
                ));
            }
            current_size += raw_property.size;
            entry_def.push(raw_property);
            match current_size.cmp(&entry_size) {
                Ordering::Greater => {
                    return Err(format_error!(
                        &format!(
                            "Sum of property size ({}) cannot exceed the entry size ({})",
                            current_size, entry_size
                        ),
                        stream
                    ))
                }
                Ordering::Equal => {
                    variants.push(Rc::new(Variant::new(entry_def)?));
                    entry_def = Vec::new();
                    current_size = 0;
                }
                Ordering::Less => {
                    /* Noting to do */
                    continue;
                }
            }
        }
        if !entry_def.is_empty() {
            variants.push(Rc::new(Variant::new(entry_def)?));
        }
        if variants.len() != variant_count.into_usize() {
            return Err(format_error!(
                &format!(
                    "Entry declare ({}) variants but properties define ({})",
                    variant_count,
                    variants.len()
                ),
                stream
            ));
        }
        Ok(Entry {
            variants,
            size: Size::from(entry_size),
        })
    }
}

impl Entry {
    pub fn create_entry(&self, reader: &Reader) -> Result<LazyEntry> {
        let variant_id = if self.variants.len() > 1 {
            reader.read_u8(Offset::zero())?
        } else {
            0
        };
        let variant_def = &self.variants[variant_id as usize];
        Ok(LazyEntry::new(
            variant_id,
            Rc::clone(variant_def),
            reader.create_sub_reader(Offset::zero(), End::None),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ContentAddress;
    use crate::reader::directory_pack::Array;
    use crate::reader::directory_pack::EntryTrait;
    use crate::reader::{Content, RawValue};

    #[test]
    fn create_entry() {
        let entry_def = Entry {
            variants: vec![Rc::new(
                Variant::new(vec![
                    RawProperty::new(RawPropertyKind::ContentAddress(0), 4),
                    RawProperty::new(RawPropertyKind::UnsignedInt, 2),
                ])
                .unwrap(),
            )],
            size: Size::new(6),
        };

        {
            let content = vec![0x00, 0x00, 0x00, 0x01, 0x88, 0x99];

            let reader = Reader::new(content, End::None);
            let entry = entry_def.create_entry(&reader).unwrap();

            assert!(entry.get_variant_id() == 0);
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
            let entry = entry_def.create_entry(&reader).unwrap();

            assert!(entry.get_variant_id() == 0);
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
        let entry_def = Entry {
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

        {
            let content = vec![0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99];

            let reader = Reader::new(content, End::None);
            let entry = entry_def.create_entry(&reader).unwrap();

            assert!(entry.get_variant_id() == 0);
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Array(Array::new(vec![0xFF, 0xEE, 0xDD, 0xCC], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::U16(0x8899));
        }

        {
            let content = vec![0x01, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99];

            let reader = Reader::new(content, End::None);
            let entry = entry_def.create_entry(&reader).unwrap();

            assert!(entry.get_variant_id() == 1);
            assert!(
                entry.get_value(0.into()).unwrap()
                    == RawValue::Array(Array::new(vec![0xFF, 0xEE], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == RawValue::I8(-52));
            assert!(entry.get_value(2.into()).unwrap() == RawValue::U16(0x8899));
        }
    }
}