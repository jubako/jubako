use crate::bases::*;
use crate::common::ContentAddress;
use crate::reader::directory_pack::layout;
use crate::reader::directory_pack::raw_value::{Array, Extend, RawValue};
use std::io::BorrowedBuf;
use std::io::Read;
use std::marker::PhantomData;

pub trait PropertyBuilderTrait {
    type Output;
    fn create(&self, reader: SubReader) -> Result<Self::Output>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Property<OutType> {
    offset: Offset,
    phantom: PhantomData<OutType>,
}

impl<OutType> Property<OutType> {
    pub fn new(offset: Offset) -> Self {
        Self {
            offset,
            phantom: PhantomData,
        }
    }
}

impl PropertyBuilderTrait for Property<u8> {
    type Output = u8;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        reader.read_u8(self.offset)
    }
}

impl PropertyBuilderTrait for Property<u16> {
    type Output = u16;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        reader.read_u16(self.offset)
    }
}

impl PropertyBuilderTrait for Property<u32> {
    type Output = u32;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        reader.read_u32(self.offset)
    }
}

impl PropertyBuilderTrait for Property<u64> {
    type Output = u64;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        reader.read_u64(self.offset)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IntProperty {
    offset: Offset,
    size: ByteSize,
}

impl IntProperty {
    pub fn new(offset: Offset, size: ByteSize) -> Self {
        Self { offset, size }
    }
}

impl TryFrom<&layout::Property> for IntProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::UnsignedInt(size) => Ok(IntProperty::new(p.offset, size)),
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for IntProperty {
    type Output = u64;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        Ok(match self.size {
            ByteSize::U1 => reader.read_u8(self.offset)? as u64,
            ByteSize::U2 => reader.read_u16(self.offset)? as u64,
            ByteSize::U3 | ByteSize::U4 => reader.read_usized(self.offset, self.size)?,
            ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                reader.read_usized(self.offset, self.size)?
            }
        })
    }
}

impl PropertyBuilderTrait for Property<i8> {
    type Output = i8;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        reader.read_i8(self.offset)
    }
}

impl PropertyBuilderTrait for Property<i16> {
    type Output = i16;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        reader.read_i16(self.offset)
    }
}

impl PropertyBuilderTrait for Property<i32> {
    type Output = i32;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        reader.read_i32(self.offset)
    }
}

impl PropertyBuilderTrait for Property<i64> {
    type Output = i64;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        reader.read_i64(self.offset)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SignedProperty {
    offset: Offset,
    size: ByteSize,
}

impl SignedProperty {
    pub fn new(offset: Offset, size: ByteSize) -> Self {
        Self { offset, size }
    }
}

impl TryFrom<&layout::Property> for SignedProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::SignedInt(size) => Ok(SignedProperty::new(p.offset, size)),
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for SignedProperty {
    type Output = i64;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        Ok(match self.size {
            ByteSize::U1 => reader.read_i8(self.offset)? as i64,
            ByteSize::U2 => reader.read_i16(self.offset)? as i64,
            ByteSize::U3 | ByteSize::U4 => reader.read_isized(self.offset, self.size)?,
            ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                reader.read_isized(self.offset, self.size)?
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArrayProperty {
    base: Option<(Offset, Size)>,
    vl_array: Option<(Offset, ValueStoreIdx, ByteSize)>,
}

impl ArrayProperty {
    pub fn new_array(offset: Offset, size: Size) -> Self {
        Self {
            base: Some((offset, size)),
            vl_array: None,
        }
    }

    pub fn new_vlarray(offset: Offset, store_id: ValueStoreIdx, size: ByteSize) -> Self {
        Self {
            base: None,
            vl_array: Some((offset, store_id, size)),
        }
    }

    pub fn new(
        offset: Offset,
        store_id: ValueStoreIdx,
        vl_size: ByteSize,
        base_size: Size,
    ) -> Self {
        Self {
            base: Some((offset + vl_size as usize, base_size)),
            vl_array: Some((offset, store_id, vl_size)),
        }
    }

    fn create_array(&self, reader: SubReader) -> Result<Vec<u8>> {
        Ok(match self.base {
            None => vec![],
            Some((offset, size)) => {
                let mut flux = reader.create_flux(offset, End::Size(size));
                let mut ret = Vec::with_capacity(size.into_usize());
                let mut uninit: BorrowedBuf = ret.spare_capacity_mut().into();
                flux.read_buf_exact(uninit.unfilled())?;
                unsafe {
                    ret.set_len(size.into_usize());
                }
                ret
            }
        })
    }
}

impl TryFrom<&layout::Property> for ArrayProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::Array(size) => {
                Ok(ArrayProperty::new_array(p.offset, size.into()))
            }
            layout::PropertyKind::VLArray(vl_size, store_id, base) => Ok(match base {
                None => ArrayProperty::new_vlarray(p.offset, store_id, vl_size),
                Some(base_size) => {
                    ArrayProperty::new(p.offset, store_id, vl_size, base_size.into())
                }
            }),
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for ArrayProperty {
    type Output = Array;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        let base = self.create_array(reader)?;
        Ok(match self.vl_array {
            None => Array::new(base, None),
            Some((offset, store_id, size)) => {
                let value_id = reader.read_usized(offset, size)?.into();
                Array::new(base, Some(Extend::new(store_id, value_id)))
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContentProperty {
    offset: Offset,
}

impl ContentProperty {
    pub fn new(offset: Offset) -> Self {
        Self { offset }
    }
    fn create_content(offset: Offset, reader: SubReader) -> Result<ContentAddress> {
        let mut flux = reader.create_flux(offset, End::new_size(4u64));
        let contentaddress = ContentAddress::produce(&mut flux)?;
        Ok(contentaddress)
    }
}

impl TryFrom<&layout::Property> for ContentProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::ContentAddress => Ok(ContentProperty::new(p.offset)),
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for ContentProperty {
    type Output = ContentAddress;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
        Self::create_content(self.offset, reader)
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
        match p.kind {
            layout::PropertyKind::ContentAddress => {
                Self::ContentAddress(ContentProperty::new(p.offset))
            }
            layout::PropertyKind::UnsignedInt(size) => {
                Self::UnsignedInt(IntProperty::new(p.offset, size))
            }
            layout::PropertyKind::SignedInt(size) => {
                Self::SignedInt(SignedProperty::new(p.offset, size))
            }
            layout::PropertyKind::Array(size) => {
                Self::Array(ArrayProperty::new_array(p.offset, size.into()))
            }
            layout::PropertyKind::VLArray(vl_size, store_id, base) => Self::Array(match base {
                None => ArrayProperty::new_vlarray(p.offset, store_id, vl_size),
                Some(base_size) => {
                    ArrayProperty::new(p.offset, store_id, vl_size, base_size.into())
                }
            }),
            layout::PropertyKind::None => unreachable!(),
        }
    }
}

impl PropertyBuilderTrait for AnyProperty {
    type Output = RawValue;
    fn create(&self, reader: SubReader) -> Result<Self::Output> {
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
