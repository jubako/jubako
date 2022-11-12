mod property;
mod variant;

// Reuse from super to allow sub module to use it.
use super::raw_layout::RawLayout;
use super::raw_value::{Array, Extend, RawValue};
use crate::bases::*;

pub use property::{Property, PropertyKind};
pub use variant::Variant;

use std::cmp::Ordering;
use std::rc::Rc;

#[derive(Debug)]
pub struct Layout {
    pub variants: Vec<Rc<Variant>>,
    pub size: Size,
}

impl Producable for Layout {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let entry_size = stream.read_u16()? as usize;
        let variant_count: VariantCount = Count::<u8>::produce(stream)?.into();
        let raw_layout = RawLayout::produce(stream)?;
        let mut variants = Vec::new();
        let mut entry_def = Vec::new();
        let mut current_size = 0;
        for raw_property in raw_layout.iter() {
            if raw_property.is_variant_id() && !entry_def.is_empty() {
                return Err(format_error!(
                    "VariantId cannot appear in the middle of a entry.",
                    stream
                ));
            }
            current_size += raw_property.size;
            entry_def.push(*raw_property);
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
        Ok(Self {
            variants,
            size: Size::from(entry_size),
        })
    }
}
