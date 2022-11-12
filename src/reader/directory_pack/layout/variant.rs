use super::super::raw_layout::{RawProperty, RawPropertyKind};
use super::property::{Property, PropertyKind};
use crate::bases::*;

#[derive(Debug)]
pub struct Variant {
    pub properties: Vec<Property>,
}

impl Variant {
    pub fn new(raw_properties: Vec<RawProperty>) -> Result<Self> {
        let mut offset = 0;
        let mut current_idx = 0;
        let mut properties = Vec::new();
        while current_idx < raw_properties.len() {
            let (property, new_idx, new_offset) =
                Self::build_property(current_idx, offset, &raw_properties)?;
            offset = new_offset;
            current_idx = new_idx;
            if property.kind != PropertyKind::None {
                properties.push(property)
            }
        }
        Ok(Self { properties })
    }
    fn build_property(
        current_idx: usize,
        offset: usize,
        raw_properties: &[RawProperty],
    ) -> Result<(Property, usize /*new_idx*/, usize /*new_offset*/)> {
        let raw_property = raw_properties[current_idx];
        match raw_property.kind {
            RawPropertyKind::VariantId => {
                if current_idx == 0 {
                    Ok((
                        Property::new(offset, PropertyKind::None),
                        current_idx + 1,
                        offset + 1,
                    ))
                } else {
                    Err(format_error!(
                        "VariantId cannot be in the middle of variant layout"
                    ))
                }
            }
            RawPropertyKind::Padding => Ok((
                Property::new(offset, PropertyKind::None),
                current_idx + 1,
                offset + raw_property.size,
            )),
            RawPropertyKind::ContentAddress(nb_base) => Ok((
                Property::new(offset, PropertyKind::ContentAddress(nb_base)),
                current_idx + 1,
                offset + (nb_base as usize + 1) * 4,
            )),
            RawPropertyKind::UnsignedInt => Ok((
                Property::new(offset, PropertyKind::UnsignedInt(raw_property.size)),
                current_idx + 1,
                offset + raw_property.size,
            )),
            RawPropertyKind::SignedInt => Ok((
                Property::new(offset, PropertyKind::SignedInt(raw_property.size)),
                current_idx + 1,
                offset + raw_property.size,
            )),
            RawPropertyKind::Array => Ok((
                Property::new(offset, PropertyKind::Array(raw_property.size)),
                current_idx + 1,
                offset + raw_property.size,
            )),
            RawPropertyKind::VLArray(flookup, valuestoreid) => {
                let (subproperty, new_idx, new_offset) = if flookup {
                    let subproperty = Self::build_property(
                        current_idx + 1,
                        offset + raw_property.size,
                        raw_properties,
                    )?;
                    let subproperty_size = if let PropertyKind::Array(s) = subproperty.0.kind {
                        s
                    } else {
                        return Err(format_error!(
                            "Lookup VLArray property must be followed by a Array property."
                        ));
                    };
                    (Some(subproperty_size), subproperty.1, subproperty.2)
                } else {
                    (None, current_idx + 1, offset + raw_property.size)
                };
                Ok((
                    Property::new(
                        offset,
                        PropertyKind::VLArray(raw_property.size, valuestoreid.into(), subproperty),
                    ),
                    new_idx,
                    new_offset,
                ))
            }
        }
    }
}
