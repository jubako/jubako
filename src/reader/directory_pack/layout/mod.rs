mod properties;
mod property;

// Reuse from super to allow sub module to use it.
use super::raw_layout::RawLayout;
use crate::bases::*;

pub use super::raw_layout::{DeportedInfo, PropertyKind};
pub use properties::{Properties, SharedProperties};
pub use property::Property;
use std::collections::HashMap;

use std::cmp::Ordering;

#[derive(Debug)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize))]
pub struct VariantPart {
    pub variant_id_offset: Offset,
    pub variants: Box<[SharedProperties]>,
    pub names: HashMap<String, u8>,
}

#[derive(Debug)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize))]
pub struct Layout {
    pub common: Properties,
    pub variant_part: Option<VariantPart>,
    pub size: Size,
}

impl Parsable for Layout {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
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
        let common_properties = Properties::new(0, common_properties)?;
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
                        variants.push(Properties::new(common_size, variant_def)?.into());
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
            common: common_properties,
            variant_part,
            size: Size::from(entry_size),
        })
    }
}
