use crate::bases::*;
use crate::common::ContentAddress;
use crate::reader::directory_pack::layout;
use crate::reader::directory_pack::raw_value::{Array, Extend, RawValue};

// The properties here are pretty close from the layout::Property.
// The main difference is that layout::Property is not typed:
// The kind of the property is a enum and we have one property for all kind
// of value we can produce.
// On the other side, builder's properties are specialized: We have on kind
// of property per kind of value. This allow specialized entry to use a specific
// builder's property.

pub trait PropertyBuilderTrait {
    type Output;
    fn create(&self, reader: &SubReader) -> Result<Self::Output>;
}

pub struct VariantIdProperty {
    offset: Offset,
}

impl VariantIdProperty {
    pub fn new(offset: Offset) -> Self {
        Self { offset }
    }
}

impl PropertyBuilderTrait for VariantIdProperty {
    type Output = VariantIdx;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        Ok(reader.read_u8(self.offset)?.into())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IntProperty {
    offset: Offset,
    size: ByteSize,
    default: Option<u64>,
}

impl IntProperty {
    pub fn new(offset: Offset, size: ByteSize, default: Option<u64>) -> Self {
        Self {
            offset,
            size,
            default,
        }
    }
}

impl TryFrom<&layout::Property> for IntProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::UnsignedInt(size, default) => {
                Ok(IntProperty::new(p.offset, size, default))
            }
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for IntProperty {
    type Output = u64;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        Ok(match self.default {
            Some(v) => v,
            None => match self.size {
                ByteSize::U1 => reader.read_u8(self.offset)? as u64,
                ByteSize::U2 => reader.read_u16(self.offset)? as u64,
                ByteSize::U3 | ByteSize::U4 => reader.read_usized(self.offset, self.size)?,
                ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                    reader.read_usized(self.offset, self.size)?
                }
            },
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SignedProperty {
    offset: Offset,
    size: ByteSize,
    default: Option<i64>,
}

impl SignedProperty {
    pub fn new(offset: Offset, size: ByteSize, default: Option<i64>) -> Self {
        Self {
            offset,
            size,
            default,
        }
    }
}

impl TryFrom<&layout::Property> for SignedProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::SignedInt(size, default) => {
                Ok(SignedProperty::new(p.offset, size, default))
            }
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for SignedProperty {
    type Output = i64;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        Ok(match self.default {
            Some(v) => v,
            None => match self.size {
                ByteSize::U1 => reader.read_i8(self.offset)? as i64,
                ByteSize::U2 => reader.read_i16(self.offset)? as i64,
                ByteSize::U3 | ByteSize::U4 => reader.read_isized(self.offset, self.size)?,
                ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                    reader.read_isized(self.offset, self.size)?
                }
            },
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArrayProperty {
    offset: Offset,
    array_size_size: Option<ByteSize>,
    fixed_array_size: u8,
    deported_array_info: Option<(ByteSize, ValueStoreIdx)>,
    default: Option<(u64, [u8; 31], Option<u64>)>,
}

impl ArrayProperty {
    pub fn new(
        offset: Offset,
        array_size_size: Option<ByteSize>,
        fixed_array_size: u8,
        deported_array_info: Option<(ByteSize, ValueStoreIdx)>,
        default: Option<(u64, [u8; 31], Option<u64>)>,
    ) -> Self {
        Self {
            offset,
            array_size_size,
            fixed_array_size,
            deported_array_info,
            default,
        }
    }
}

impl TryFrom<&layout::Property> for ArrayProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::Array(size, fixed_array_size, deported, default) => Ok(
                ArrayProperty::new(p.offset, size, fixed_array_size, deported, default),
            ),
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for ArrayProperty {
    type Output = Array;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        let (array_size, base_array, deported_info) = match self.default {
            Some((array_size, base_array, value_id)) => (
                Some(array_size.into()),
                Vec::from(&base_array[..self.fixed_array_size.into()]),
                self.deported_array_info
                    .map(|(_, store_id)| Extend::new(store_id, value_id.unwrap().into())),
            ),
            None => {
                let mut flux = reader.create_flux_from(self.offset);
                let array_size = match self.array_size_size {
                    None => None,
                    Some(size) => Some(flux.read_usized(size)?.into()),
                };
                let base_array = if self.fixed_array_size != 0 {
                    flux.read_vec(self.fixed_array_size.into())?
                } else {
                    Vec::new()
                };
                let deported_info = match self.deported_array_info {
                    Some((value_size, store_id)) => {
                        let value_id = flux.read_usized(value_size)?.into();
                        Some(Extend::new(store_id, value_id))
                    }
                    None => None,
                };
                (array_size, base_array, deported_info)
            }
        };
        Ok(Array::new(array_size, base_array, deported_info))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContentProperty {
    offset: Offset,
    content_id_size: ByteSize,
}

impl ContentProperty {
    pub fn new(offset: Offset, content_id_size: ByteSize) -> Self {
        Self {
            offset,
            content_id_size,
        }
    }
}

impl TryFrom<&layout::Property> for ContentProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::ContentAddress(s) => Ok(ContentProperty::new(p.offset, s)),
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for ContentProperty {
    type Output = ContentAddress;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        let content_size = self.content_id_size as usize + 1;
        let mut flux = reader.create_flux(self.offset, End::new_size(content_size));
        let pack_id = flux.read_u8()?;
        let content_id = flux.read_usized(self.content_id_size)? as u32;
        Ok(ContentAddress::new(pack_id.into(), content_id.into()))
    }
}

/// The definition of a property, as we need to parse it.
/// In opposition to RawProperty, the property is the "final" property.
/// It describe how to parse te value of a entry.
#[derive(Debug, PartialEq, Eq)]
pub enum AnyProperty {
    ContentAddress(ContentProperty),
    UnsignedInt(IntProperty),
    SignedInt(SignedProperty),
    Array(ArrayProperty),
}

impl From<&layout::Property> for AnyProperty {
    fn from(p: &layout::Property) -> Self {
        match &p.kind {
            &layout::PropertyKind::ContentAddress(size) => {
                Self::ContentAddress(ContentProperty::new(p.offset, size))
            }
            &layout::PropertyKind::UnsignedInt(size, default) => {
                Self::UnsignedInt(IntProperty::new(p.offset, size, default))
            }
            &layout::PropertyKind::SignedInt(size, default) => {
                Self::SignedInt(SignedProperty::new(p.offset, size, default))
            }
            layout::PropertyKind::Array(size, fixed_array_size, deported, default) => Self::Array(
                ArrayProperty::new(p.offset, *size, *fixed_array_size, *deported, *default),
            ),
            &layout::PropertyKind::DeportedUnsignedInt(_, _, _) => todo!(),
            &layout::PropertyKind::DeportedSignedInt(_, _, _) => todo!(),
            layout::PropertyKind::Padding => unreachable!(),
            layout::PropertyKind::VariantId => unreachable!(),
        }
    }
}

impl PropertyBuilderTrait for AnyProperty {
    type Output = RawValue;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        Ok(match self {
            Self::ContentAddress(prop) => RawValue::Content(prop.create(reader)?),
            Self::UnsignedInt(prop) => match prop.size {
                ByteSize::U1 => RawValue::U8(prop.create(reader)? as u8),
                ByteSize::U2 => RawValue::U16(prop.create(reader)? as u16),
                ByteSize::U3 | ByteSize::U4 => RawValue::U32(prop.create(reader)? as u32),
                ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                    RawValue::U64(prop.create(reader)?)
                }
            },
            Self::SignedInt(prop) => match prop.size {
                ByteSize::U1 => RawValue::I8(prop.create(reader)? as i8),
                ByteSize::U2 => RawValue::I16(prop.create(reader)? as i16),
                ByteSize::U3 | ByteSize::U4 => RawValue::I32(prop.create(reader)? as i32),
                ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                    RawValue::I64(prop.create(reader)?)
                }
            },
            Self::Array(prop) => RawValue::Array(prop.create(reader)?),
        })
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uint() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let prop = Property::new(0, PropertyKind::UnsignedInt(1));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::U8(0xFE));
        let prop = Property::new(2, PropertyKind::UnsignedInt(1));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::U8(0xBA));

        let prop = Property::new(0, PropertyKind::UnsignedInt(2));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::U16(0xFEDC));
        let prop = Property::new(2, PropertyKind::UnsignedInt(2));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::U16(0xBA98));

        let prop = Property::new(0, PropertyKind::UnsignedInt(3));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::U32(0xFEDCBA));
        let prop = Property::new(2, PropertyKind::UnsignedInt(3));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::U32(0xBA9876));

        let prop = Property::new(0, PropertyKind::UnsignedInt(4));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U32(0xFEDCBA98)
        );
        let prop = Property::new(2, PropertyKind::UnsignedInt(4));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U32(0xBA987654)
        );

        let prop = Property::new(0, PropertyKind::UnsignedInt(5));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U64(0xFEDCBA9876)
        );
        let prop = Property::new(2, PropertyKind::UnsignedInt(5));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U64(0xBA98765432)
        );

        let prop = Property::new(0, PropertyKind::UnsignedInt(6));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U64(0xFEDCBA987654)
        );
        let prop = Property::new(2, PropertyKind::UnsignedInt(6));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U64(0xBA9876543210)
        );

        let prop = Property::new(0, PropertyKind::UnsignedInt(7));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U64(0xFEDCBA98765432)
        );
        let prop = Property::new(2, PropertyKind::UnsignedInt(7));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U64(0xBA9876543210ff)
        );

        let prop = Property::new(0, PropertyKind::UnsignedInt(8));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U64(0xFEDCBA9876543210)
        );
        let prop = Property::new(1, PropertyKind::UnsignedInt(8));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::U64(0xDCBA9876543210ff)
        );
    }

    #[test]
    fn test_sint() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let prop = Property::new(0, PropertyKind::SignedInt(1));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::I8(-0x02));
        let prop = Property::new(2, PropertyKind::SignedInt(1));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::I8(-0x46));

        let prop = Property::new(0, PropertyKind::SignedInt(2));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::I16(-0x0124));
        let prop = Property::new(2, PropertyKind::SignedInt(2));
        assert_eq!(prop.create_value(&reader).unwrap(), RawValue::I16(-0x4568));

        let prop = Property::new(0, PropertyKind::SignedInt(3));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I32(-0x012346)
        );
        let prop = Property::new(2, PropertyKind::SignedInt(3));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I32(-0x45678a)
        );

        let prop = Property::new(0, PropertyKind::SignedInt(4));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I32(-0x01234568)
        );
        let prop = Property::new(2, PropertyKind::SignedInt(4));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I32(-0x456789ac)
        );

        let prop = Property::new(0, PropertyKind::SignedInt(5));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I64(-0x012345678a)
        );
        let prop = Property::new(2, PropertyKind::SignedInt(5));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I64(-0x456789abce)
        );

        let prop = Property::new(0, PropertyKind::SignedInt(6));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I64(-0x0123456789ac)
        );
        let prop = Property::new(2, PropertyKind::SignedInt(6));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I64(-0x456789abcdf0)
        );

        let prop = Property::new(0, PropertyKind::SignedInt(7));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I64(-0x0123456789abce)
        );
        let prop = Property::new(2, PropertyKind::SignedInt(7));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I64(-0x456789abcdef01)
        );

        let prop = Property::new(0, PropertyKind::SignedInt(8));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I64(-0x0123456789abcdf0)
        );
        let prop = Property::new(1, PropertyKind::SignedInt(8));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::I64(-0x23456789abcdef01)
        );
    }

    #[test]
    fn test_array() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let prop = Property::new(0, PropertyKind::Array(1));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xFE), None))
        );
        let prop = Property::new(2, PropertyKind::Array(1));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xBA), None))
        );

        let prop = Property::new(0, PropertyKind::Array(2));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xFE, 0xDC), None))
        );
        let prop = Property::new(2, PropertyKind::Array(2));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xBA, 0x98), None))
        );

        let prop = Property::new(0, PropertyKind::Array(3));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xFE, 0xDC, 0xBA), None))
        );
        let prop = Property::new(2, PropertyKind::Array(3));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xBA, 0x98, 0x76), None))
        );

        let prop = Property::new(0, PropertyKind::Array(8));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10),
                None
            ))
        );
    }

    #[test]
    fn test_vlarray() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let prop = Property::new(0, PropertyKind::VLArray(1, Idx::from(255), None));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(Idx::from(255), 0xFE))
            ))
        );
        let prop = Property::new(2, PropertyKind::VLArray(1, Idx::from(255), None));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(Idx::from(255), 0xBA))
            ))
        );

        let prop = Property::new(0, PropertyKind::VLArray(2, Idx::from(255), None));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(Idx::from(255), 0xFEDC))
            ))
        );
        let prop = Property::new(2, PropertyKind::VLArray(2, Idx::from(255), None));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(Idx::from(255), 0xBA98))
            ))
        );

        let prop = Property::new(0, PropertyKind::VLArray(1, Idx::from(255), Some(1)));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0xDC),
                Some(Extend::new(Idx::from(255), 0xFE))
            ))
        );
        let prop = Property::new(2, PropertyKind::VLArray(1, Idx::from(255), Some(1)));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98),
                Some(Extend::new(Idx::from(255), 0xBA))
            ))
        );

        let prop = Property::new(0, PropertyKind::VLArray(1, Idx::from(255), Some(3)));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0xDC, 0xBA, 0x98),
                Some(Extend::new(Idx::from(255), 0xFE))
            ))
        );
        let prop = Property::new(2, PropertyKind::VLArray(1, Idx::from(255), Some(3)));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98, 0x76, 0x54),
                Some(Extend::new(Idx::from(255), 0xBA))
            ))
        );

        let prop = Property::new(0, PropertyKind::VLArray(3, Idx::from(255), Some(3)));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98, 0x76, 0x54),
                Some(Extend::new(Idx::from(255), 0xFEDCBA))
            ))
        );
        let prop = Property::new(2, PropertyKind::VLArray(3, Idx::from(255), Some(3)));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x54, 0x32, 0x10),
                Some(Extend::new(Idx::from(255), 0xBA9876))
            ))
        );
    }

    #[test]
    fn test_content() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let prop = Property::new(0, PropertyKind::ContentAddress(0));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Content(Content::new(
                ContentAddress {
                    pack_id: Id(0xFE),
                    content_id: Idx(0xDCBA98)
                },
                None
            ))
        );
        let prop = Property::new(2, PropertyKind::ContentAddress(0));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Content(Content::new(
                ContentAddress {
                    pack_id: Id(0xBA),
                    content_id: Idx(0x987654)
                },
                None
            ))
        );
    }
}
*/
