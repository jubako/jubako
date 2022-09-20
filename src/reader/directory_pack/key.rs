use super::raw_value::{Array, Content, Extend, RawValue};
use crate::bases::*;
use crate::common::ContentAddress;
use std::io::BorrowedBuf;

// The kind of the key. This will be the descriminant to how parse the value.
#[derive(Debug, PartialEq, Eq)]
pub enum KeyKind {
    ContentAddress(u8),
    UnsignedInt(usize),
    SignedInt(usize),
    CharArray(usize),
    PString(usize, Idx<u8>, Option<usize>),
    None,
}

/// The definition of a key, as we need to parse it.
/// In opposition to KeyDef, the key is the "final" key.
/// It describe how to parse te value of a entry.
#[derive(Debug, PartialEq, Eq)]
pub struct Key {
    offset: Offset,
    pub kind: KeyKind,
}

impl Key {
    pub fn new(offset: usize, kind: KeyKind) -> Self {
        Self {
            offset: Offset(offset as u64),
            kind,
        }
    }

    fn create_content(offset: Offset, base: u8, reader: &dyn Reader) -> Result<Content> {
        let mut stream = reader.create_stream(offset, End::Size(Size(4)));
        let contentaddress = ContentAddress::produce(stream.as_mut())?;
        let base_content = if base == 0 {
            None
        } else {
            Some(Key::create_content(Offset(offset.0 + 4), base - 1, reader)?)
        };
        Ok(Content::new(contentaddress, base_content))
    }

    fn create_array(offset: Offset, size: usize, reader: &dyn Reader) -> Result<Vec<u8>> {
        let mut stream = reader.create_stream(offset, End::Size(size.into()));
        let mut ret = Vec::with_capacity(size);
        let mut uninit: BorrowedBuf = ret.spare_capacity_mut().into();
        stream.read_buf_exact(uninit.unfilled())?;
        unsafe {
            ret.set_len(size);
        }
        Ok(ret)
    }

    pub fn create_value(&self, reader: &dyn Reader) -> Result<RawValue> {
        Ok(match &self.kind {
            KeyKind::ContentAddress(base) => {
                RawValue::Content(Key::create_content(self.offset, *base, reader)?)
            }
            KeyKind::UnsignedInt(size) => match size {
                1 => RawValue::U8(reader.read_u8(self.offset)?),
                2 => RawValue::U16(reader.read_u16(self.offset)?),
                3 | 4 => RawValue::U32(reader.read_usized(self.offset, *size)? as u32),
                5 | 6 | 7 | 8 => RawValue::U64(reader.read_usized(self.offset, *size)?),
                _ => unreachable!(),
            },
            KeyKind::SignedInt(size) => match size {
                1 => RawValue::I8(reader.read_i8(self.offset)?),
                2 => RawValue::I16(reader.read_i16(self.offset)?),
                3 | 4 => RawValue::I32(reader.read_isized(self.offset, *size)? as i32),
                5 | 6 | 7 | 8 => RawValue::I64(reader.read_isized(self.offset, *size)?),
                _ => unreachable!(),
            },
            KeyKind::CharArray(size) => RawValue::Array(Array::new(
                Key::create_array(self.offset, *size, reader)?,
                None,
            )),
            KeyKind::PString(size, store_id, base) => {
                let key_id = reader.read_usized(self.offset, *size)?;
                let base = match base {
                    None => Vec::new(),
                    Some(base_size) => Key::create_array(
                        Offset(self.offset.0 + (*size as u64)),
                        *base_size,
                        reader,
                    )?,
                };
                RawValue::Array(Array::new(base, Some(Extend::new(*store_id, key_id))))
            }
            KeyKind::None => unreachable!(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uint() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let key = Key::new(0, KeyKind::UnsignedInt(1));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::U8(0xFE));
        let key = Key::new(2, KeyKind::UnsignedInt(1));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::U8(0xBA));

        let key = Key::new(0, KeyKind::UnsignedInt(2));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::U16(0xFEDC));
        let key = Key::new(2, KeyKind::UnsignedInt(2));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::U16(0xBA98));

        let key = Key::new(0, KeyKind::UnsignedInt(3));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::U32(0xFEDCBA));
        let key = Key::new(2, KeyKind::UnsignedInt(3));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::U32(0xBA9876));

        let key = Key::new(0, KeyKind::UnsignedInt(4));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U32(0xFEDCBA98)
        );
        let key = Key::new(2, KeyKind::UnsignedInt(4));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U32(0xBA987654)
        );

        let key = Key::new(0, KeyKind::UnsignedInt(5));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U64(0xFEDCBA9876)
        );
        let key = Key::new(2, KeyKind::UnsignedInt(5));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U64(0xBA98765432)
        );

        let key = Key::new(0, KeyKind::UnsignedInt(6));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U64(0xFEDCBA987654)
        );
        let key = Key::new(2, KeyKind::UnsignedInt(6));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U64(0xBA9876543210)
        );

        let key = Key::new(0, KeyKind::UnsignedInt(7));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U64(0xFEDCBA98765432)
        );
        let key = Key::new(2, KeyKind::UnsignedInt(7));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U64(0xBA9876543210ff)
        );

        let key = Key::new(0, KeyKind::UnsignedInt(8));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U64(0xFEDCBA9876543210)
        );
        let key = Key::new(1, KeyKind::UnsignedInt(8));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::U64(0xDCBA9876543210ff)
        );
    }

    #[test]
    fn test_sint() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let key = Key::new(0, KeyKind::SignedInt(1));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::I8(-0x02));
        let key = Key::new(2, KeyKind::SignedInt(1));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::I8(-0x46));

        let key = Key::new(0, KeyKind::SignedInt(2));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::I16(-0x0124));
        let key = Key::new(2, KeyKind::SignedInt(2));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::I16(-0x4568));

        let key = Key::new(0, KeyKind::SignedInt(3));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::I32(-0x012346));
        let key = Key::new(2, KeyKind::SignedInt(3));
        assert_eq!(key.create_value(&reader).unwrap(), RawValue::I32(-0x45678a));

        let key = Key::new(0, KeyKind::SignedInt(4));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I32(-0x01234568)
        );
        let key = Key::new(2, KeyKind::SignedInt(4));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I32(-0x456789ac)
        );

        let key = Key::new(0, KeyKind::SignedInt(5));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I64(-0x012345678a)
        );
        let key = Key::new(2, KeyKind::SignedInt(5));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I64(-0x456789abce)
        );

        let key = Key::new(0, KeyKind::SignedInt(6));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I64(-0x0123456789ac)
        );
        let key = Key::new(2, KeyKind::SignedInt(6));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I64(-0x456789abcdf0)
        );

        let key = Key::new(0, KeyKind::SignedInt(7));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I64(-0x0123456789abce)
        );
        let key = Key::new(2, KeyKind::SignedInt(7));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I64(-0x456789abcdef01)
        );

        let key = Key::new(0, KeyKind::SignedInt(8));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I64(-0x0123456789abcdf0)
        );
        let key = Key::new(1, KeyKind::SignedInt(8));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::I64(-0x23456789abcdef01)
        );
    }

    #[test]
    fn test_chararray() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let key = Key::new(0, KeyKind::CharArray(1));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xFE), None))
        );
        let key = Key::new(2, KeyKind::CharArray(1));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xBA), None))
        );

        let key = Key::new(0, KeyKind::CharArray(2));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xFE, 0xDC), None))
        );
        let key = Key::new(2, KeyKind::CharArray(2));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xBA, 0x98), None))
        );

        let key = Key::new(0, KeyKind::CharArray(3));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xFE, 0xDC, 0xBA), None))
        );
        let key = Key::new(2, KeyKind::CharArray(3));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(vec!(0xBA, 0x98, 0x76), None))
        );

        let key = Key::new(0, KeyKind::CharArray(8));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10),
                None
            ))
        );
    }

    #[test]
    fn test_pstring() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = BufReader::new(content, End::None);
        let key = Key::new(0, KeyKind::PString(1, Idx::from(255), None));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(Idx::from(255), 0xFE))
            ))
        );
        let key = Key::new(2, KeyKind::PString(1, Idx::from(255), None));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(Idx::from(255), 0xBA))
            ))
        );

        let key = Key::new(0, KeyKind::PString(2, Idx::from(255), None));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(Idx::from(255), 0xFEDC))
            ))
        );
        let key = Key::new(2, KeyKind::PString(2, Idx::from(255), None));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                Vec::new(),
                Some(Extend::new(Idx::from(255), 0xBA98))
            ))
        );

        let key = Key::new(0, KeyKind::PString(1, Idx::from(255), Some(1)));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0xDC),
                Some(Extend::new(Idx::from(255), 0xFE))
            ))
        );
        let key = Key::new(2, KeyKind::PString(1, Idx::from(255), Some(1)));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98),
                Some(Extend::new(Idx::from(255), 0xBA))
            ))
        );

        let key = Key::new(0, KeyKind::PString(1, Idx::from(255), Some(3)));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0xDC, 0xBA, 0x98),
                Some(Extend::new(Idx::from(255), 0xFE))
            ))
        );
        let key = Key::new(2, KeyKind::PString(1, Idx::from(255), Some(3)));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98, 0x76, 0x54),
                Some(Extend::new(Idx::from(255), 0xBA))
            ))
        );

        let key = Key::new(0, KeyKind::PString(3, Idx::from(255), Some(3)));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Array(Array::new(
                vec!(0x98, 0x76, 0x54),
                Some(Extend::new(Idx::from(255), 0xFEDCBA))
            ))
        );
        let key = Key::new(2, KeyKind::PString(3, Idx::from(255), Some(3)));
        assert_eq!(
            key.create_value(&reader).unwrap(),
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
        let key = Key::new(0, KeyKind::ContentAddress(0));
        assert_eq!(
            key.create_value(&reader).unwrap(),
            RawValue::Content(Content::new(
                ContentAddress {
                    pack_id: Id(0xFE),
                    content_id: Idx(0xDCBA98)
                },
                None
            ))
        );
        let key = Key::new(2, KeyKind::ContentAddress(0));
        assert_eq!(
            key.create_value(&reader).unwrap(),
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
