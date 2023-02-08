mod properties;
mod property;

// Reuse from super to allow sub module to use it.
use super::raw_layout::RawLayout;
use super::raw_value::{Array, Extend, RawValue};
use crate::bases::*;

pub use properties::{Properties, SharedProperties};
pub use property::{Property, PropertyKind};

use std::cmp::Ordering;

type VariantPart = (Offset, Box<[SharedProperties]>);

#[derive(Debug)]
pub struct Layout {
    pub common: Properties,
    pub variant_part: Option<VariantPart>,
    pub size: Size,
}

impl Producable for Layout {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let entry_size = flux.read_u16()? as usize;
        let variant_count: VariantCount = Count::<u8>::produce(flux)?.into();
        let raw_layout = RawLayout::produce(flux)?;
        let mut common_properties = Vec::new();
        let mut common_size = 0;
        let mut property_iter = raw_layout.iter().peekable();
        while let Some(raw_property) = property_iter.next_if(|p| !p.is_variant_id()) {
            common_size += raw_property.size;
            common_properties.push(*raw_property);
        }
        let common_properties = Properties::new(0, common_properties)?;
        let variant_part = if variant_count.into_u8() != 0 {
            let variant_id_offset = Offset::from(common_size);
            common_size += 1;

            let mut variant_size = 0;
            let mut variants = Vec::new();
            let mut variant_def = Vec::new();
            let mut variant_started = false;
            for raw_property in property_iter {
                if !raw_property.is_variant_id() && !variant_started {
                    return Err(format_error!(
                        "Variant definition must start with a VariantId.",
                        flux
                    ));
                }
                if raw_property.is_variant_id() && variant_started {
                    return Err(format_error!(
                        "VariantId cannot be in the middle of a variant definition.",
                        flux
                    ));
                }
                if raw_property.is_variant_id() {
                    // This is a special property
                    variant_started = true;
                    continue;
                }
                variant_size += raw_property.size;
                variant_def.push(*raw_property);
                match variant_size.cmp(&(entry_size - common_size)) {
                    Ordering::Greater => {
                        return Err(format_error!(
                            &format!(
                                "Sum of variant size ({common_size} + {variant_size}) cannot exceed the entry size ({entry_size})"
                            ),
                            flux
                        ))
                    }
                    Ordering::Equal => {
                        variants.push(Properties::new(common_size, variant_def)?.into());
                        variant_def = Vec::new();
                        variant_size = 0;
                        variant_started = false;
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
                    flux
                ));
            }
            Some((variant_id_offset, variants.into_boxed_slice()))
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
