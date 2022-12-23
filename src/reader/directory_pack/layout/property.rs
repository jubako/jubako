use super::{Array, Extend, RawValue};
use crate::bases::*;
use crate::common::{Content, ContentAddress};
use std::io::{BorrowedBuf, Read};

// The kind of the property. This will be the descriminant to how parse the value.
#[derive(Debug, PartialEq, Eq)]
pub enum PropertyKind {
    ContentAddress(u8),
    UnsignedInt(usize),
    SignedInt(usize),
    Array(usize),
    VLArray(usize, ValueStoreIdx, Option<usize>),
    None,
}

/// The definition of a property, as we need to parse it.
/// In opposition to RawProperty, the property is the "final" property.
/// It describe how to parse te value of a entry.
#[derive(Debug, PartialEq, Eq)]
pub struct Property {
    offset: Offset,
    pub kind: PropertyKind,
}

impl Property {
    pub fn new(offset: usize, kind: PropertyKind) -> Self {
        Self {
            offset: Offset::from(offset),
            kind,
        }
    }

    fn create_content(offset: Offset, base: u8, reader: &Reader) -> Result<Content> {
        let mut stream = reader.create_stream(offset, End::new_size(4));
        let contentaddress = ContentAddress::produce(&mut stream)?;
        let base_content = if base == 0 {
            None
        } else {
            Some(Property::create_content(offset + 4, base - 1, reader)?)
        };
        Ok(Content::new(contentaddress, base_content))
    }

    fn create_array(offset: Offset, size: usize, reader: &Reader) -> Result<Vec<u8>> {
        let mut stream = reader.create_stream(offset, End::new_size(size as u64));
        let mut ret = Vec::with_capacity(size);
        let mut uninit: BorrowedBuf = ret.spare_capacity_mut().into();
        stream.read_buf_exact(uninit.unfilled())?;
        unsafe {
            ret.set_len(size);
        }
        Ok(ret)
    }

    pub fn create_value(&self, reader: &Reader) -> Result<RawValue> {
        Ok(match &self.kind {
            PropertyKind::ContentAddress(base) => {
                RawValue::Content(Property::create_content(self.offset, *base, reader)?)
            }
            PropertyKind::UnsignedInt(size) => match size {
                1 => RawValue::U8(reader.read_u8(self.offset)?),
                2 => RawValue::U16(reader.read_u16(self.offset)?),
                3 | 4 => RawValue::U32(reader.read_usized(self.offset, *size)? as u32),
                5 | 6 | 7 | 8 => RawValue::U64(reader.read_usized(self.offset, *size)?),
                _ => unreachable!(),
            },
            PropertyKind::SignedInt(size) => match size {
                1 => RawValue::I8(reader.read_i8(self.offset)?),
                2 => RawValue::I16(reader.read_i16(self.offset)?),
                3 | 4 => RawValue::I32(reader.read_isized(self.offset, *size)? as i32),
                5 | 6 | 7 | 8 => RawValue::I64(reader.read_isized(self.offset, *size)?),
                _ => unreachable!(),
            },
            PropertyKind::Array(size) => RawValue::Array(Array::new(
                Property::create_array(self.offset, *size, reader)?,
                None,
            )),
            PropertyKind::VLArray(size, store_id, base) => {
                let value_id = reader.read_usized(self.offset, *size)?;
                let base = match base {
                    None => Vec::new(),
                    Some(base_size) => {
                        Property::create_array(self.offset + (*size), *base_size, reader)?
                    }
                };
                RawValue::Array(Array::new(
                    base,
                    Some(Extend::new(*store_id, ValueIdx::from(value_id))),
                ))
            }
            PropertyKind::None => unreachable!(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uint() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = Reader::new(content, End::None);
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
        let reader = Reader::new(content, End::None);
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
        let reader = Reader::new(content, End::None);
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
        let reader = Reader::new(content, End::None);
        let prop = Property::new(0, PropertyKind::VLArray(1, ValueStoreIdx::from(255), None));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(ValueStoreIdx::from(255), ValueIdx::from(0xFE)))
            ))
        );
        let prop = Property::new(2, PropertyKind::VLArray(1, ValueStoreIdx::from(255), None));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(ValueStoreIdx::from(255), ValueIdx::from(0xBA)))
            ))
        );

        let prop = Property::new(0, PropertyKind::VLArray(2, ValueStoreIdx::from(255), None));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(
                    ValueStoreIdx::from(255),
                    ValueIdx::from(0xFEDC)
                ))
            ))
        );
        let prop = Property::new(2, PropertyKind::VLArray(2, ValueStoreIdx::from(255), None));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(
                    ValueStoreIdx::from(255),
                    ValueIdx::from(0xBA98)
                ))
            ))
        );

        let prop = Property::new(
            0,
            PropertyKind::VLArray(1, ValueStoreIdx::from(255), Some(1)),
        );
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0xDC),
                Some(Extend::new(ValueStoreIdx::from(255), ValueIdx::from(0xFE)))
            ))
        );
        let prop = Property::new(
            2,
            PropertyKind::VLArray(1, ValueStoreIdx::from(255), Some(1)),
        );
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98),
                Some(Extend::new(ValueStoreIdx::from(255), ValueIdx::from(0xBA)))
            ))
        );

        let prop = Property::new(
            0,
            PropertyKind::VLArray(1, ValueStoreIdx::from(255), Some(3)),
        );
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0xDC, 0xBA, 0x98),
                Some(Extend::new(ValueStoreIdx::from(255), ValueIdx::from(0xFE)))
            ))
        );
        let prop = Property::new(
            2,
            PropertyKind::VLArray(1, ValueStoreIdx::from(255), Some(3)),
        );
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98, 0x76, 0x54),
                Some(Extend::new(ValueStoreIdx::from(255), ValueIdx::from(0xBA)))
            ))
        );

        let prop = Property::new(
            0,
            PropertyKind::VLArray(3, ValueStoreIdx::from(255), Some(3)),
        );
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98, 0x76, 0x54),
                Some(Extend::new(
                    ValueStoreIdx::from(255),
                    ValueIdx::from(0xFEDCBA)
                ))
            ))
        );
        let prop = Property::new(
            2,
            PropertyKind::VLArray(3, ValueStoreIdx::from(255), Some(3)),
        );
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x54, 0x32, 0x10),
                Some(Extend::new(
                    ValueStoreIdx::from(255),
                    ValueIdx::from(0xBA9876)
                ))
            ))
        );
    }

    #[test]
    fn test_content() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = Reader::new(content, End::None);
        let prop = Property::new(0, PropertyKind::ContentAddress(0));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Content(Content::new(
                ContentAddress {
                    pack_id: PackId::from(0xFE),
                    content_id: ContentIdx::from(0xDCBA98)
                },
                None
            ))
        );
        let prop = Property::new(2, PropertyKind::ContentAddress(0));
        assert_eq!(
            prop.create_value(&reader).unwrap(),
            RawValue::Content(Content::new(
                ContentAddress {
                    pack_id: PackId::from(0xBA),
                    content_id: ContentIdx::from(0x987654)
                },
                None
            ))
        );
    }
}