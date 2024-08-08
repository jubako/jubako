use crate::bases::*;
use crate::common::ContentAddress;
use crate::reader::directory_pack::layout;
use crate::reader::directory_pack::private::ValueStorageTrait;
use crate::reader::directory_pack::raw_layout::{DeportedDefault, DeportedInfo, PropertyKind};
use crate::reader::directory_pack::raw_value::{Array, Extend, RawValue};
use crate::reader::directory_pack::ValueStoreTrait;
use std::sync::Arc;
use zerocopy::byteorder::{ByteOrder, LittleEndian as LE};

// The properties here are pretty close from the layout::Property.
// The main difference is that layout::Property is not typed:
// The kind of the property is a enum and we have one property for all kind
// of value we can produce.
// On the other side, builder's properties are specialized: We have on kind
// of property per kind of value. This allow specialized entry to use a specific
// builder's property.

pub trait PropertyBuilderTrait {
    type Output;
    fn create(&self, parser: &impl RandomParser) -> Result<Self::Output>;
}

#[derive(Debug, Copy, Clone)]
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
    fn create(&self, parser: &impl RandomParser) -> Result<Self::Output> {
        VariantIdx::rparse(parser, self.offset)
    }
}

#[derive(Debug, Clone)]
pub struct IntProperty {
    offset: Offset,
    size: ByteSize,
    default: Option<u64>,
    deported: Option<(ByteSize, Arc<dyn ValueStoreTrait>)>,
}

impl IntProperty {
    pub(self) fn new(
        offset: Offset,
        size: ByteSize,
        default: Option<u64>,
        deported: Option<(ByteSize, Arc<dyn ValueStoreTrait>)>,
    ) -> Self {
        Self {
            offset,
            size,
            default,
            deported,
        }
    }

    fn new_from_deported(
        offset: Offset,
        size: ByteSize,
        store: Arc<dyn ValueStoreTrait>,
        deported_default: DeportedDefault,
    ) -> Result<Self> {
        match deported_default {
            DeportedDefault::Value(default) => {
                let default_data =
                    store.get_data(default.into(), Some(ASize::from(size as u8 as usize)))?;
                let default = LE::read_uint(default_data, size as u8 as usize);
                Ok(IntProperty::new(offset, size, Some(default), None))
            }
            DeportedDefault::KeySize(key_size) => Ok(IntProperty::new(
                offset,
                size,
                None,
                Some((key_size, store)),
            )),
        }
    }
}

impl<ValueStorage: ValueStorageTrait> TryFrom<(&layout::Property, &ValueStorage)> for IntProperty {
    type Error = Error;
    fn try_from(
        p_vs: (&layout::Property, &ValueStorage),
    ) -> std::result::Result<Self, Self::Error> {
        let (p, value_storage) = p_vs;
        match p.kind {
            PropertyKind::UnsignedInt { int_size, default } => {
                Ok(IntProperty::new(p.offset, int_size, default, None))
            }
            PropertyKind::DeportedUnsignedInt {
                int_size,
                value_store_idx,
                id,
            } => {
                let store = value_storage.get_value_store(value_store_idx)?;
                IntProperty::new_from_deported(p.offset, int_size, store, id)
            }
            _ => Err("Invalid key".to_string().into()),
        }
    }
}

impl PropertyBuilderTrait for IntProperty {
    type Output = u64;
    fn create(&self, parser: &impl RandomParser) -> Result<Self::Output> {
        Ok(match self.default {
            Some(v) => v,
            None => match &self.deported {
                Some((key_size, value_store)) => {
                    let key = match key_size {
                        ByteSize::U1 => parser.read_u8(self.offset)? as u64,
                        ByteSize::U2 => parser.read_u16(self.offset)? as u64,
                        ByteSize::U4 => parser.read_u32(self.offset)? as u64,
                        ByteSize::U8 => parser.read_u64(self.offset)?,
                        _ => parser.read_usized(self.offset, *key_size)?,
                    };
                    let value_data = value_store
                        .get_data(key.into(), Some(ASize::from(self.size as u8 as usize)))?;
                    LE::read_uint(value_data, self.size as u8 as usize)
                }
                None => match self.size {
                    ByteSize::U1 => parser.read_u8(self.offset)? as u64,
                    ByteSize::U2 => parser.read_u16(self.offset)? as u64,
                    ByteSize::U4 => parser.read_u32(self.offset)? as u64,
                    ByteSize::U8 => parser.read_u64(self.offset)?,
                    _ => parser.read_usized(self.offset, self.size)?,
                },
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct SignedProperty {
    offset: Offset,
    size: ByteSize,
    default: Option<i64>,
    deported: Option<(ByteSize, Arc<dyn ValueStoreTrait>)>,
}

impl SignedProperty {
    pub(self) fn new(
        offset: Offset,
        size: ByteSize,
        default: Option<i64>,
        deported: Option<(ByteSize, Arc<dyn ValueStoreTrait>)>,
    ) -> Self {
        Self {
            offset,
            size,
            default,
            deported,
        }
    }

    fn new_from_deported(
        offset: Offset,
        size: ByteSize,
        store: Arc<dyn ValueStoreTrait>,
        deported_default: DeportedDefault,
    ) -> Result<Self> {
        match deported_default {
            DeportedDefault::Value(default) => {
                let default_data =
                    store.get_data(default.into(), Some(ASize::from(size as u8 as usize)))?;
                let default = LE::read_int(default_data, size as u8 as usize);
                Ok(SignedProperty::new(offset, size, Some(default), None))
            }
            DeportedDefault::KeySize(key_size) => Ok(SignedProperty::new(
                offset,
                size,
                None,
                Some((key_size, store)),
            )),
        }
    }
}

impl<ValueStorage: ValueStorageTrait> TryFrom<(&layout::Property, &ValueStorage)>
    for SignedProperty
{
    type Error = Error;
    fn try_from(
        p_vs: (&layout::Property, &ValueStorage),
    ) -> std::result::Result<Self, Self::Error> {
        let (p, value_storage) = p_vs;
        match p.kind {
            layout::PropertyKind::SignedInt { int_size, default } => {
                Ok(SignedProperty::new(p.offset, int_size, default, None))
            }
            PropertyKind::DeportedSignedInt {
                int_size,
                value_store_idx,
                id,
            } => {
                let store = value_storage.get_value_store(value_store_idx)?;
                SignedProperty::new_from_deported(p.offset, int_size, store, id)
            }
            _ => Err("Invalid key".to_string().into()),
        }
    }
}

impl PropertyBuilderTrait for SignedProperty {
    type Output = i64;
    fn create(&self, parser: &impl RandomParser) -> Result<Self::Output> {
        Ok(match self.default {
            Some(v) => v,
            None => match &self.deported {
                Some((key_size, value_store)) => {
                    let key = match key_size {
                        ByteSize::U1 => parser.read_u8(self.offset)? as u64,
                        ByteSize::U2 => parser.read_u16(self.offset)? as u64,
                        ByteSize::U4 => parser.read_u32(self.offset)? as u64,
                        ByteSize::U8 => parser.read_u64(self.offset)?,
                        _ => parser.read_usized(self.offset, *key_size)?,
                    };
                    let value_data = value_store
                        .get_data(key.into(), Some(ASize::from(self.size as u8 as usize)))?;
                    LE::read_int(value_data, self.size as u8 as usize)
                }
                None => match self.size {
                    ByteSize::U1 => parser.read_i8(self.offset)? as i64,
                    ByteSize::U2 => parser.read_i16(self.offset)? as i64,
                    ByteSize::U4 => parser.read_i32(self.offset)? as i64,
                    ByteSize::U8 => parser.read_i64(self.offset)?,
                    _ => parser.read_isized(self.offset, self.size)?,
                },
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct ArrayProperty {
    offset: Offset,
    array_len_size: Option<ByteSize>,
    fixed_array_len: u8,
    deported_array_info: Option<(ByteSize, Arc<dyn ValueStoreTrait>)>,
    default: Option<(ASize, BaseArray, Option<u64>)>,
}

impl ArrayProperty {
    pub(self) fn new(
        offset: Offset,
        array_len_size: Option<ByteSize>,
        fixed_array_len: u8,
        deported_array_info: Option<(ByteSize, Arc<dyn ValueStoreTrait>)>,
        default: Option<(ASize, BaseArray, Option<u64>)>,
    ) -> Self {
        Self {
            offset,
            array_len_size,
            fixed_array_len,
            deported_array_info,
            default,
        }
    }
}

impl<ValueStorage: ValueStorageTrait> TryFrom<(&layout::Property, &ValueStorage)>
    for ArrayProperty
{
    type Error = Error;
    fn try_from(
        p_vs: (&layout::Property, &ValueStorage),
    ) -> std::result::Result<Self, Self::Error> {
        let (p, value_storage) = p_vs;
        match p.kind {
            layout::PropertyKind::Array {
                array_len_size,
                fixed_array_len,
                deported_info,
                default,
            } => {
                let deported_info = match deported_info {
                    None => None,
                    Some(DeportedInfo {
                        id_size,
                        value_store_idx,
                    }) => {
                        let value_store = value_storage.get_value_store(value_store_idx)?;
                        Some((id_size, value_store as Arc<dyn ValueStoreTrait>))
                    }
                };
                Ok(ArrayProperty::new(
                    p.offset,
                    array_len_size,
                    fixed_array_len,
                    deported_info,
                    default,
                ))
            }
            _ => Err("Invalid key".to_string().into()),
        }
    }
}

impl PropertyBuilderTrait for ArrayProperty {
    type Output = Array;
    fn create(&self, parser: &impl RandomParser) -> Result<Self::Output> {
        let (array_size, base_array, deported_info) = match self.default {
            Some((array_size, base_array, value_id)) => (
                Some(array_size),
                base_array,
                self.deported_array_info
                    .as_ref()
                    .map(|(_, store)| Extend::new(Arc::clone(store), value_id.unwrap().into())),
            ),
            None => {
                let mut seq_parser = parser.create_parser(self.offset)?;
                let array_size = match self.array_len_size {
                    None => None,
                    Some(size) => {
                        assert!(size as u8 <= 3);
                        let array_size = seq_parser.read_usized(size)?;
                        Some(ASize::new(array_size as usize))
                    }
                };
                let base_array = BaseArray::parse(self.fixed_array_len, &mut seq_parser)?;
                let deported_info = match &self.deported_array_info {
                    Some((value_size, store)) => {
                        let value_id = seq_parser.read_usized(*value_size)?.into();
                        Some(Extend::new(Arc::clone(store), value_id))
                    }
                    None => None,
                };
                (array_size, base_array, deported_info)
            }
        };
        Ok(Array::new(
            array_size,
            base_array,
            self.fixed_array_len,
            deported_info,
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ContentProperty {
    offset: Offset,
    pack_id_default: Option<PackId>,
    pack_id_size: ByteSize,
    content_id_size: ByteSize,
}

impl ContentProperty {
    pub(self) fn new(
        offset: Offset,
        pack_id_default: Option<PackId>,
        pack_id_size: ByteSize,
        content_id_size: ByteSize,
    ) -> Self {
        Self {
            offset,
            pack_id_default,
            pack_id_size,
            content_id_size,
        }
    }
}

impl TryFrom<&layout::Property> for ContentProperty {
    type Error = String;
    fn try_from(p: &layout::Property) -> std::result::Result<Self, Self::Error> {
        match p.kind {
            layout::PropertyKind::ContentAddress {
                pack_id_size,
                content_id_size,
                default_pack_id,
            } => Ok(ContentProperty::new(
                p.offset,
                default_pack_id,
                pack_id_size,
                content_id_size,
            )),
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for ContentProperty {
    type Output = ContentAddress;
    fn create(&self, parser: &impl RandomParser) -> Result<Self::Output> {
        let mut seq_parser = parser.create_parser(self.offset)?;
        let pack_id = match self.pack_id_default {
            None => (seq_parser.read_usized(self.pack_id_size)? as u16).into(),
            Some(d) => d,
        };

        let content_id = seq_parser.read_usized(self.content_id_size)? as u32;
        Ok(ContentAddress::new(pack_id, content_id.into()))
    }
}

/// The definition of a property, as we need to parse it.
/// In opposition to RawProperty, the property is the "final" property.
/// It describe how to parse te value of a entry.
#[derive(Debug)]
pub enum AnyProperty {
    ContentAddress(ContentProperty),
    UnsignedInt(IntProperty),
    SignedInt(SignedProperty),
    Array(ArrayProperty),
}

impl<ValueStorage: ValueStorageTrait> TryFrom<(&layout::Property, &ValueStorage)> for AnyProperty {
    type Error = Error;
    fn try_from(
        p_vs: (&layout::Property, &ValueStorage),
    ) -> std::result::Result<Self, Self::Error> {
        let (p, value_storage) = p_vs;
        Ok(match &p.kind {
            &PropertyKind::ContentAddress {
                pack_id_size,
                content_id_size,
                default_pack_id,
            } => Self::ContentAddress(ContentProperty::new(
                p.offset,
                default_pack_id,
                pack_id_size,
                content_id_size,
            )),
            &PropertyKind::UnsignedInt { int_size, default } => {
                Self::UnsignedInt(IntProperty::new(p.offset, int_size, default, None))
            }
            &PropertyKind::SignedInt { int_size, default } => {
                Self::SignedInt(SignedProperty::new(p.offset, int_size, default, None))
            }
            PropertyKind::Array {
                array_len_size,
                fixed_array_len,
                deported_info,
                default,
            } => {
                let deported = match deported_info {
                    None => None,
                    Some(DeportedInfo {
                        id_size,
                        value_store_idx,
                    }) => {
                        let value_store = value_storage.get_value_store(*value_store_idx)?;
                        Some((*id_size, value_store as Arc<dyn ValueStoreTrait>))
                    }
                };
                Self::Array(ArrayProperty::new(
                    p.offset,
                    *array_len_size,
                    *fixed_array_len,
                    deported,
                    *default,
                ))
            }
            &PropertyKind::DeportedUnsignedInt {
                int_size,
                value_store_idx,
                id,
            } => {
                let store = value_storage.get_value_store(value_store_idx)?;
                Self::UnsignedInt(IntProperty::new_from_deported(
                    p.offset, int_size, store, id,
                )?)
            }
            &PropertyKind::DeportedSignedInt {
                int_size,
                value_store_idx,
                id,
            } => {
                let store = value_storage.get_value_store(value_store_idx)?;
                Self::SignedInt(SignedProperty::new_from_deported(
                    p.offset, int_size, store, id,
                )?)
            }
            PropertyKind::Padding => unreachable!(),
            PropertyKind::VariantId => unreachable!(),
        })
    }
}

impl PropertyBuilderTrait for AnyProperty {
    type Output = RawValue;
    fn create(&self, parser: &impl RandomParser) -> Result<Self::Output> {
        Ok(match self {
            Self::ContentAddress(prop) => RawValue::Content(prop.create(parser)?),
            Self::UnsignedInt(prop) => match prop.size {
                ByteSize::U1 => RawValue::U8(prop.create(parser)? as u8),
                ByteSize::U2 => RawValue::U16(prop.create(parser)? as u16),
                ByteSize::U3 | ByteSize::U4 => RawValue::U32(prop.create(parser)? as u32),
                ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                    RawValue::U64(prop.create(parser)?)
                }
            },
            Self::SignedInt(prop) => match prop.size {
                ByteSize::U1 => RawValue::I8(prop.create(parser)? as i8),
                ByteSize::U2 => RawValue::I16(prop.create(parser)? as i16),
                ByteSize::U3 | ByteSize::U4 => RawValue::I32(prop.create(parser)? as i32),
                ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                    RawValue::I64(prop.create(parser)?)
                }
            },
            Self::Array(prop) => RawValue::Array(prop.create(parser)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uint() {
        let reader = Reader::from([0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());
        let prop = IntProperty::new(Offset::new(0), ByteSize::U1, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0xFE);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U1, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0xBA);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U2, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0xDCFE);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U2, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x98BA);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U3, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0xBADCFE);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U3, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x7698BA);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U4, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x98BADCFE);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U4, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x547698BA);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U5, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x7698BADCFE);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U5, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x32547698BA);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U6, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x547698BADCFE);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U6, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x1032547698BA);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U7, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x32547698BADCFE);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U7, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0xFF1032547698BA);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U8, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x1032547698BADCFE);
        let prop = IntProperty::new(Offset::new(1), ByteSize::U8, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0xFF1032547698BADC);
    }

    #[test]
    fn test_sint() {
        let reader = Reader::from([0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());
        let prop = SignedProperty::new(Offset::new(0), ByteSize::U1, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -0x02);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U1, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -0x46);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U2, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -0x2302);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U2, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -0x6746);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U3, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -0x452302);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U3, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x7698BA);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U4, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -0x67452302);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U4, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x547698BA);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U5, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x7698BADCFE);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U5, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x32547698BA);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U6, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x547698BADCFE);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U6, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x1032547698BA);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U7, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x32547698BADCFE);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U7, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -0x00efcdab896746);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U8, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x1032547698BADCFE);
        let prop = SignedProperty::new(Offset::new(1), ByteSize::U8, None, None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -0x00efcdab89674524);
    }

    #[test]
    fn test_default_int() {
        let reader = Reader::from([]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());
        let prop = IntProperty::new(Offset::new(0), ByteSize::U1, Some(50), None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 50);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U2, Some(500), None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 500);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U1, Some(-50), None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -50);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U1, Some(50), None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 50);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U2, Some(500), None);
        assert_eq!(prop.create(&byte_slice).unwrap(), 500);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U2, Some(-500), None);
        assert_eq!(prop.create(&byte_slice).unwrap(), -500);
    }

    mod mock {
        use super::*;
        #[derive(Debug)]
        pub struct ValueStore {
            data: Vec<u8>,
        }
        impl ValueStore {
            pub fn new() -> Self {
                let data = (0..255).collect();
                Self { data }
            }
        }
        impl ValueStoreTrait for ValueStore {
            fn get_data(&self, idx: ValueIdx, size: Option<ASize>) -> Result<&[u8]> {
                let idx = idx.into_u64() as usize;
                if idx < 100 {
                    let end = idx + size.unwrap().into_usize();
                    Ok(&self.data[idx..end])
                } else if idx >= 1000 && idx < 1100 {
                    let idx = idx - 900;
                    let end = idx + size.unwrap().into_usize();
                    Ok(&self.data[idx..end])
                } else {
                    unreachable!()
                }
            }
        }
    }

    #[test]
    fn test_deported_int() {
        let value_store = Arc::new(mock::ValueStore::new());
        let reader = Reader::from([0x01, 0x00, 0x02, 0x03, 0x04, 0x1A, 0x04, 0x20, 0x04, 0xff]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())), // Size of the key
        ); // key is 0x01
        assert_eq!(prop.create(&byte_slice).unwrap(), 1);

        let prop = IntProperty::new(
            Offset::new(2),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x02
        assert_eq!(prop.create(&byte_slice).unwrap(), 2);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0001
        assert_eq!(prop.create(&byte_slice).unwrap(), 1);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x01
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x0201);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0001
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x0201);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U3, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x001
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x030201);

        let prop = IntProperty::new(
            Offset::new(2),
            ByteSize::U3, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x02
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x040302);

        let prop = IntProperty::new(
            Offset::new(4),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x04
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x04);

        let prop = IntProperty::new(
            Offset::new(4),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x04
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x0504);

        let prop = IntProperty::new(
            Offset::new(5),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x041A
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x96);

        let prop = IntProperty::new(
            Offset::new(5),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x041A
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x9796);

        let prop = IntProperty::new(
            Offset::new(7),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0420
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x9C);

        let prop = IntProperty::new(
            Offset::new(7),
            ByteSize::U8, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0420
        assert_eq!(prop.create(&byte_slice).unwrap(), 0xA3A2A1A09F9E9D9C);

        let prop = IntProperty::new(
            Offset::new(7),
            ByteSize::U8, // Size of the integer
            Some(20),
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0420 (but we don't care as we have a default value of 20)
        assert_eq!(prop.create(&byte_slice).unwrap(), 20);
    }

    #[test]
    fn test_deported_sint() {
        let value_store = Arc::new(mock::ValueStore::new());
        let reader = Reader::from([0x01, 0x00, 0x02, 0x03, 0x04, 0x1A, 0x04, 0x20, 0x04, 0xff]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())), // Size of the key
        ); // key is 0x01
        assert_eq!(prop.create(&byte_slice).unwrap(), 1);

        let prop = SignedProperty::new(
            Offset::new(2),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x02
        assert_eq!(prop.create(&byte_slice).unwrap(), 2);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0001
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x01);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x01
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x0201);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0001
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x0201);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U3, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x01
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x030201);

        let prop = SignedProperty::new(
            Offset::new(2),
            ByteSize::U3, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x02
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x040302);

        let prop = SignedProperty::new(
            Offset::new(4),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x04
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x04);

        let prop = SignedProperty::new(
            Offset::new(4),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        ); // key is 0x04
        assert_eq!(prop.create(&byte_slice).unwrap(), 0x0504);

        let prop = SignedProperty::new(
            Offset::new(5),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x041A
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            -106_i64 //0x96_i8 overflowing
        );

        let prop = SignedProperty::new(
            Offset::new(5),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x041A
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            -0x686A_i64 // 0x9796_i16 overflowing
        );

        let prop = SignedProperty::new(
            Offset::new(7),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); //  key is 0x0420
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            -100_i64 // 0x9C_i8 overflowing
        );

        let prop = SignedProperty::new(
            Offset::new(7),
            ByteSize::U8, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0420
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            -0x5C5D5E5F60616264_i64 // 0xA3A2A1A09F9E9D9C overflowing
        );

        let prop = SignedProperty::new(
            Offset::new(7),
            ByteSize::U8, // Size of the integer
            Some(-20),
            Some((ByteSize::U2, value_store.clone())),
        ); // key is 0x0420 (but we don't care as we have a default value of 20)
        assert_eq!(prop.create(&byte_slice).unwrap(), -20);
    }

    #[derive(Debug)]
    struct FakeArray {
        size: Option<ASize>,
        base: BaseArray,
        base_len: u8,
        extend: Option<ValueIdx>,
    }

    impl FakeArray {
        fn new(
            size: Option<ASize>,
            base: BaseArray,
            base_len: u8,
            extend: Option<ValueIdx>,
        ) -> Self {
            Self {
                size,
                base,
                base_len,
                extend,
            }
        }
    }

    impl PartialEq<Array> for FakeArray {
        fn eq(&self, other: &Array) -> bool {
            let base = self.size == other.size
                && self.base == other.base
                && self.base_len == other.base_len;
            if !base {
                return false;
            }
            if self.extend.is_some() != other.extend.is_some() {
                return false;
            }
            if self.extend.is_none() {
                return true;
            }
            self.extend.unwrap() == other.extend.as_ref().unwrap().value_id
        }
    }

    #[test]
    fn test_array() {
        let reader = Reader::from([0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());
        let prop = ArrayProperty::new(Offset::new(0), Some(ByteSize::U1), 1, None, None);
        assert_eq!(
            FakeArray::new(Some(ASize::new(0xFE)), BaseArray::new(&[0xDC]), 1, None),
            prop.create(&byte_slice).unwrap()
        );
        let prop = ArrayProperty::new(Offset::new(2), Some(ByteSize::U1), 1, None, None);
        assert_eq!(
            FakeArray::new(Some(ASize::new(0xBA)), BaseArray::new(&[0x98]), 1, None),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(Offset::new(0), Some(ByteSize::U1), 2, None, None);
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA]),
                2,
                None
            ),
            prop.create(&byte_slice).unwrap(),
        );
        let prop = ArrayProperty::new(Offset::new(2), Some(ByteSize::U1), 2, None, None);
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xBA)),
                BaseArray::new(&[0x98, 0x76]),
                2,
                None
            ),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(Offset::new(0), Some(ByteSize::U1), 3, None, None);
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA, 0x98]),
                3,
                None
            ),
            prop.create(&byte_slice).unwrap(),
        );
        let prop = ArrayProperty::new(Offset::new(2), Some(ByteSize::U1), 3, None, None);
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xBA)),
                BaseArray::new(&[0x98, 0x76, 0x54]),
                3,
                None
            ),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(Offset::new(0), Some(ByteSize::U1), 8, None, None);
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff]),
                8,
                None
            ),
            prop.create(&byte_slice).unwrap(),
        );
    }

    mod fake {
        use super::*;
        use crate::reader::directory_pack::ValueStoreTrait;
        #[derive(Debug)]
        pub struct ValueStore {}

        impl ValueStoreTrait for ValueStore {
            fn get_data(&self, _id: ValueIdx, _size: Option<ASize>) -> Result<&[u8]> {
                unreachable!()
            }
        }
    }

    #[test]
    fn test_deported_array() {
        let reader = Reader::from([0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());
        let value_store = Arc::new(fake::ValueStore {}) as Arc<dyn ValueStoreTrait>;
        let prop = ArrayProperty::new(
            Offset::new(0),                                 // offset on the entry
            Some(ByteSize::U1),                             // Size of the size of the array
            0,                                              // fixed array size
            Some((ByteSize::U1, Arc::clone(&value_store))), // Size of the key
            None,                                           // default
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xFE)),
                BaseArray::default(),
                0,
                Some(0xDC.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U1),
            0,
            Some((ByteSize::U1, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xBA)),
                BaseArray::default(),
                0,
                Some(0x98.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(
            Offset::new(0),
            Some(ByteSize::U1),
            0,
            Some((ByteSize::U2, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xFE)),
                BaseArray::default(),
                0,
                Some(0xBADC.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );
        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U1),
            0,
            Some((ByteSize::U2, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xBA)),
                BaseArray::default(),
                0,
                Some(0x7698.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(
            Offset::new(0),
            Some(ByteSize::U1),
            1,
            Some((ByteSize::U1, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xFE)),
                BaseArray::new(&[0xDC]),
                1,
                Some(0xBA.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );
        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U1),
            1,
            Some((ByteSize::U1, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xBA)),
                BaseArray::new(&[0x98]),
                1,
                Some(0x76.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(
            Offset::new(0),
            Some(ByteSize::U1),
            3,
            Some((ByteSize::U1, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA, 0x98]),
                3,
                Some(0x76.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );
        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U1),
            3,
            Some((ByteSize::U1, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xBA)),
                BaseArray::new(&[0x98, 0x76, 0x54]),
                3,
                Some(0x32.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(
            Offset::new(0),
            Some(ByteSize::U1),
            3,
            Some((ByteSize::U3, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA, 0x98]),
                3,
                Some(0x325476.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );
        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U1),
            3,
            Some((ByteSize::U3, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(0xBA)),
                BaseArray::new(&[0x98, 0x76, 0x54]),
                3,
                Some(0xff1032.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );
    }

    #[test]
    fn default_array() {
        let reader = Reader::from([]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());
        let value_store = Arc::new(fake::ValueStore {}) as Arc<dyn ValueStoreTrait>;

        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U1),
            3,
            None,
            Some((2.into(), BaseArray::new(&[0x01, 0x02]), None)),
        );
        assert_eq!(
            FakeArray::new(Some(ASize::new(2)), BaseArray::new(&[0x01, 0x02]), 2, None),
            prop.create(&byte_slice).unwrap(),
        );

        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U3),
            3,
            Some((ByteSize::U2, Arc::clone(&value_store))),
            Some((2000.into(), BaseArray::new(&[0x01, 0x02]), Some(300))),
        );
        assert_eq!(
            FakeArray::new(
                Some(ASize::new(2000)),
                BaseArray::new(&[0x01, 0x02, 0x00]),
                3,
                Some(300.into())
            ),
            prop.create(&byte_slice).unwrap(),
        );
    }

    #[test]
    fn test_content() {
        let reader = Reader::from([0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff]);
        let byte_slice = reader.get_byte_slice(Offset::zero(), reader.size());

        let prop = ContentProperty::new(Offset::new(0), None, ByteSize::U1, ByteSize::U3);
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            ContentAddress::new(PackId::from(0xFE), ContentIdx::from(0x98BADC))
        );

        let prop = ContentProperty::new(Offset::new(2), None, ByteSize::U1, ByteSize::U3);
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            ContentAddress::new(PackId::from(0xBA), ContentIdx::from(0x547698))
        );

        let prop = ContentProperty::new(Offset::new(2), None, ByteSize::U1, ByteSize::U2);
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            ContentAddress::new(PackId::from(0xBA), ContentIdx::from(0x7698))
        );

        let prop = ContentProperty::new(
            Offset::new(2),
            Some(PackId::from(0xFF)),
            ByteSize::U1,
            ByteSize::U3,
        );
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            ContentAddress::new(PackId::from(0xFF), ContentIdx::from(0x7698BA))
        );

        let prop = ContentProperty::new(
            Offset::new(2),
            Some(PackId::from(0xFF)),
            ByteSize::U1,
            ByteSize::U1,
        );
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            ContentAddress::new(PackId::from(0xFF), ContentIdx::from(0xBA))
        );

        let prop = ContentProperty::new(Offset::new(0), None, ByteSize::U2, ByteSize::U3);
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            ContentAddress::new(PackId::from(0xDCFE), ContentIdx::from(0x7698BA))
        );

        let prop = ContentProperty::new(Offset::new(2), None, ByteSize::U2, ByteSize::U3);
        assert_eq!(
            prop.create(&byte_slice).unwrap(),
            ContentAddress::new(PackId::from(0x98BA), ContentIdx::from(0x325476))
        );
    }
}
