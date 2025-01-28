mod properties;
mod property;

use super::builder::{VariantIdBuilder, VariantIdProperty};
// Reuse from super to allow sub module to use it.
use super::raw_layout::RawLayout;
use crate::bases::*;

#[cfg(feature = "explorable")]
pub(super) use super::raw_layout::DeportedDefault;
pub(super) use super::raw_layout::PropertyKind;

pub(crate) use properties::{Properties, SharedProperties};
pub(crate) use property::Property;
use std::collections::HashMap;

use std::cmp::Ordering;

#[derive(Debug)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub struct VariantPart {
    pub variant_id_offset: Offset,
    pub variants: Box<[SharedProperties]>,
    pub names: HashMap<String, u8>,
}

impl VariantPart {
    pub fn get(&self, name: impl VariantName) -> Option<&SharedProperties> {
        self.names
            .get(name.as_str())
            .and_then(|idx| self.variants.get(*idx as usize))
    }

    pub fn len(&self) -> usize {
        self.variants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    pub fn as_builder(&self) -> VariantIdProperty {
        VariantIdProperty::new(self.variant_id_offset)
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for VariantPart {
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        out.field("variant_id_offset", &self.variant_id_offset.into_u64())?;
        let display_map = self
            .names
            .iter()
            .map(|(k, v)| (k.to_string(), self.variants[*v as usize].clone()))
            .collect::<HashMap<_, _>>();
        out.field("Variants", &display_map)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub struct Layout {
    pub(crate) entry_count: EntryCount,
    pub(crate) is_entry_checked: bool,
    pub common: Properties,
    pub variant_part: Option<VariantPart>,
    pub(crate) entry_size: ASize,
}

impl Layout {
    pub fn get_variant(&self, name: impl VariantName) -> Option<&SharedProperties> {
        self.variant_part.as_ref().and_then(|v| v.get(name))
    }
    pub fn variant_len(&self) -> usize {
        self.variant_part.as_ref().map_or(0, |v| v.len())
    }
    pub fn variant_id_builder<T>(&self) -> Option<VariantIdBuilder<T>>
    where
        T: for<'a> TryFrom<&'a str> + Copy,
    {
        self.variant_part.as_ref().map(|v_part| {
            let mut variant_map = v_part
                .names
                .iter()
                .map(|(key, idx)| (*idx, T::try_from(key).ok()))
                .collect::<Vec<_>>();
            variant_map.sort_by_key(|(idx, _)| *idx);
            let variant_map = variant_map
                .into_iter()
                .map(|(_, variant)| variant)
                .collect::<Vec<_>>();
            let raw_variant_id_builder = v_part.as_builder();
            VariantIdBuilder::new(raw_variant_id_builder, variant_map)
        })
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for Layout {
    fn header_footer(&self) -> Option<(String, String)> {
        Some(("Layout(".to_string(), "".to_string()))
    }
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        out.field("entry count", &self.entry_count.into_u64())?;
        out.field("is_entry_checked", &self.is_entry_checked)?;
        out.field("entry_size", &self.entry_size)?;
        out.field("common part", &self.common)?;
        self.variant_part.print(out)
    }
}

impl Parsable for Layout {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let entry_count: EntryCount = Count::<u32>::parse(parser)?.into();
        let flag = parser.read_u8()?;
        let is_entry_checked = (flag & 0b0000_0001) != 0;
        let entry_size = parser.read_u16()? as usize;
        let variant_count: VariantCount = Count::<u8>::parse(parser)?.into();
        let mut raw_layout = RawLayout::parse(parser)?;
        let mut common_properties = Vec::new();
        let mut common_size = 0;
        let mut property_iter = raw_layout.drain(..).peekable();
        while let Some(raw_property) = property_iter.next_if(|p| !p.is_variant_id()) {
            common_size += raw_property.size;
            common_properties.push(raw_property);
        }
        let common_properties = Properties::new(0, common_properties);
        let variant_part = if variant_count.into_u8() != 0 {
            let variant_id_offset = Offset::from(common_size);
            common_size += 1;

            let mut variant_size = 0;
            let mut variants = Vec::new();
            let mut variants_map = HashMap::new();
            let mut variant_def = Vec::new();
            let mut variant_name: Option<String> = None;
            for raw_property in property_iter {
                if !raw_property.is_variant_id() && variant_name.is_none() {
                    return Err(format_error!(
                        "Variant definition must start with a VariantId.",
                        parser
                    ));
                }
                if raw_property.is_variant_id() && variant_name.is_some() {
                    return Err(format_error!(
                        "VariantId cannot be in the middle of a variant definition.",
                        parser
                    ));
                }
                if raw_property.is_variant_id() {
                    // This is a special property
                    variant_name = raw_property.name;
                    continue;
                }
                variant_size += raw_property.size;
                variant_def.push(raw_property);
                match variant_size.cmp(&(entry_size - common_size)) {
                    Ordering::Greater => {
                        return Err(format_error!(
                            &format!(
                                "Sum of variant size ({common_size} + {variant_size}) cannot exceed the entry size ({entry_size})"
                            ),
                            parser
                        ))
                    }
                    Ordering::Equal => {
                        variants.push(Properties::new(common_size, variant_def).into());
                        variants_map.insert(variant_name.unwrap(), variants.len() as u8 - 1);
                        variant_def = Vec::new();
                        variant_size = 0;
                        variant_name = None;
                    }
                    Ordering::Less => {
                        /* Noting to do */
                        continue;
                    }
                }
            }
            if !variant_def.is_empty() {
                return Err(format_error!("We cannot have left over variant definiton."));
            }
            if variants.len() != variant_count.into_usize() {
                return Err(format_error!(
                    &format!(
                        "Entry declare ({variant_count}) variants but properties define ({})",
                        variants.len()
                    ),
                    parser
                ));
            }
            Some(VariantPart {
                variant_id_offset,
                variants: variants.into_boxed_slice(),
                names: variants_map,
            })
        } else {
            None
        };
        Ok(Self {
            entry_count,
            is_entry_checked,
            common: common_properties,
            variant_part,
            entry_size: entry_size.into(),
        })
    }
}
