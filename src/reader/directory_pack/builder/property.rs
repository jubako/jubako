use crate::bases::*;
use crate::common::ContentAddress;
use crate::reader::directory_pack::layout;
use crate::reader::directory_pack::private::ValueStorageTrait;
use crate::reader::directory_pack::raw_layout::{DeportedDefault, PropertyKind};
use crate::reader::directory_pack::raw_value::{Array, Extend, RawValue};
use crate::reader::directory_pack::ValueStoreTrait;
use std::sync::Arc;

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
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        Ok(reader.read_u8(self.offset)?.into())
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
    pub fn new(
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
                use crate::bases::primitive::read_to_u64;
                let default_data =
                    store.get_data(default.into(), Some(Size::from(size as u8 as usize)))?;
                let default = read_to_u64(size as u8 as usize, default_data);
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
            PropertyKind::UnsignedInt(size, default) => {
                Ok(IntProperty::new(p.offset, size, default, None))
            }
            PropertyKind::DeportedUnsignedInt(size, store_idx, deported_default) => {
                let store = value_storage.get_value_store(store_idx)?;
                IntProperty::new_from_deported(p.offset, size, store, deported_default)
            }
            _ => Err("Invalid key".to_string().into()),
        }
    }
}

impl PropertyBuilderTrait for IntProperty {
    type Output = u64;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        Ok(match self.default {
            Some(v) => v,
            None => match &self.deported {
                Some((key_size, value_store)) => {
                    use crate::bases::primitive::read_to_u64;
                    let key = match key_size {
                        ByteSize::U1 => reader.read_u8(self.offset)? as u64,
                        ByteSize::U2 => reader.read_u16(self.offset)? as u64,
                        ByteSize::U4 => reader.read_u32(self.offset)? as u64,
                        ByteSize::U8 => reader.read_u64(self.offset)?,
                        _ => reader.read_usized(self.offset, *key_size)?,
                    };
                    let value_data = value_store
                        .get_data(key.into(), Some(Size::from(self.size as u8 as usize)))?;
                    read_to_u64(self.size as u8 as usize, value_data)
                }
                None => match self.size {
                    ByteSize::U1 => reader.read_u8(self.offset)? as u64,
                    ByteSize::U2 => reader.read_u16(self.offset)? as u64,
                    ByteSize::U4 => reader.read_u32(self.offset)? as u64,
                    ByteSize::U8 => reader.read_u64(self.offset)?,
                    _ => reader.read_usized(self.offset, self.size)?,
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
    pub fn new(
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
                use crate::bases::primitive::read_to_i64;
                let default_data =
                    store.get_data(default.into(), Some(Size::from(size as u8 as usize)))?;
                let default = read_to_i64(size as u8 as usize, default_data);
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
            layout::PropertyKind::SignedInt(size, default) => {
                Ok(SignedProperty::new(p.offset, size, default, None))
            }
            PropertyKind::DeportedSignedInt(size, store_idx, deported_default) => {
                let store = value_storage.get_value_store(store_idx)?;
                SignedProperty::new_from_deported(p.offset, size, store, deported_default)
            }
            _ => Err("Invalid key".to_string().into()),
        }
    }
}

impl PropertyBuilderTrait for SignedProperty {
    type Output = i64;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        Ok(match self.default {
            Some(v) => v,
            None => match &self.deported {
                Some((key_size, value_store)) => {
                    use crate::bases::primitive::read_to_i64;
                    let key = match key_size {
                        ByteSize::U1 => reader.read_u8(self.offset)? as u64,
                        ByteSize::U2 => reader.read_u16(self.offset)? as u64,
                        ByteSize::U3 | ByteSize::U4 => {
                            reader.read_usized(self.offset, self.size)?
                        }
                        ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                            reader.read_usized(self.offset, self.size)?
                        }
                    };
                    let value_data = value_store
                        .get_data(key.into(), Some(Size::from(self.size as u8 as usize)))?;
                    read_to_i64(self.size as u8 as usize, value_data)
                }
                None => match self.size {
                    ByteSize::U1 => reader.read_i8(self.offset)? as i64,
                    ByteSize::U2 => reader.read_i16(self.offset)? as i64,
                    ByteSize::U3 | ByteSize::U4 => reader.read_isized(self.offset, self.size)?,
                    ByteSize::U5 | ByteSize::U6 | ByteSize::U7 | ByteSize::U8 => {
                        reader.read_isized(self.offset, self.size)?
                    }
                },
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct ArrayProperty {
    offset: Offset,
    array_size_size: Option<ByteSize>,
    fixed_array_size: u8,
    deported_array_info: Option<(ByteSize, Arc<dyn ValueStoreTrait>)>,
    default: Option<(u64, BaseArray, Option<u64>)>,
}

impl ArrayProperty {
    pub fn new(
        offset: Offset,
        array_size_size: Option<ByteSize>,
        fixed_array_size: u8,
        deported_array_info: Option<(ByteSize, Arc<dyn ValueStoreTrait>)>,
        default: Option<(u64, BaseArray, Option<u64>)>,
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

impl<ValueStorage: ValueStorageTrait> TryFrom<(&layout::Property, &ValueStorage)>
    for ArrayProperty
{
    type Error = Error;
    fn try_from(
        p_vs: (&layout::Property, &ValueStorage),
    ) -> std::result::Result<Self, Self::Error> {
        let (p, value_storage) = p_vs;
        match p.kind {
            layout::PropertyKind::Array(size, fixed_array_size, deported, default) => {
                let deported = match deported {
                    None => None,
                    Some((size, store_id)) => {
                        let value_store = value_storage.get_value_store(store_id)?;
                        Some((size, value_store as Arc<dyn ValueStoreTrait>))
                    }
                };
                Ok(ArrayProperty::new(
                    p.offset,
                    size,
                    fixed_array_size,
                    deported,
                    default,
                ))
            }
            _ => Err("Invalid key".to_string().into()),
        }
    }
}

impl PropertyBuilderTrait for ArrayProperty {
    type Output = Array;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        let (array_size, base_array, deported_info) = match self.default {
            Some((array_size, base_array, value_id)) => (
                Some(array_size.into()),
                base_array,
                self.deported_array_info
                    .as_ref()
                    .map(|(_, store)| Extend::new(Arc::clone(store), value_id.unwrap().into())),
            ),
            None => {
                let mut flux = reader.create_flux_from(self.offset);
                let array_size = match self.array_size_size {
                    None => None,
                    Some(size) => Some(flux.read_usized(size)?.into()),
                };
                let base_array = BaseArray::new_from_flux(self.fixed_array_size, &mut flux)?;
                let deported_info = match &self.deported_array_info {
                    Some((value_size, store)) => {
                        let value_id = flux.read_usized(*value_size)?.into();
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
            self.fixed_array_size,
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
    pub fn new(
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
            layout::PropertyKind::ContentAddress(
                pack_id_size,
                content_id_size,
                pack_id_default,
            ) => Ok(ContentProperty::new(
                p.offset,
                pack_id_default,
                pack_id_size,
                content_id_size,
            )),
            _ => Err("Invalid key".to_string()),
        }
    }
}

impl PropertyBuilderTrait for ContentProperty {
    type Output = ContentAddress;
    fn create(&self, reader: &SubReader) -> Result<Self::Output> {
        let content_size =
            self.content_id_size as usize + if self.pack_id_default.is_some() { 0 } else { 1 };
        let mut flux = reader.create_flux(self.offset, End::new_size(content_size));
        let pack_id = match self.pack_id_default {
            None => (flux.read_usized(self.pack_id_size)? as u16).into(),
            Some(d) => d,
        };

        let content_id = flux.read_usized(self.content_id_size)? as u32;
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
            &PropertyKind::ContentAddress(pack_id_size, content_id_size, pack_id_default) => {
                Self::ContentAddress(ContentProperty::new(
                    p.offset,
                    pack_id_default,
                    pack_id_size,
                    content_id_size,
                ))
            }
            &PropertyKind::UnsignedInt(size, default) => {
                Self::UnsignedInt(IntProperty::new(p.offset, size, default, None))
            }
            &PropertyKind::SignedInt(size, default) => {
                Self::SignedInt(SignedProperty::new(p.offset, size, default, None))
            }
            PropertyKind::Array(size, fixed_array_size, deported, default) => {
                let deported = match deported {
                    None => None,
                    Some((size, store_id)) => {
                        let value_store = value_storage.get_value_store(*store_id)?;
                        Some((*size, value_store as Arc<dyn ValueStoreTrait>))
                    }
                };
                Self::Array(ArrayProperty::new(
                    p.offset,
                    *size,
                    *fixed_array_size,
                    deported,
                    *default,
                ))
            }
            &PropertyKind::DeportedUnsignedInt(size, store_id, deported_default) => {
                let store = value_storage.get_value_store(store_id)?;
                Self::UnsignedInt(IntProperty::new_from_deported(
                    p.offset,
                    size,
                    store,
                    deported_default,
                )?)
            }
            &PropertyKind::DeportedSignedInt(size, store_id, deported_default) => {
                let store = value_storage.get_value_store(store_id)?;
                Self::SignedInt(SignedProperty::new_from_deported(
                    p.offset,
                    size,
                    store,
                    deported_default,
                )?)
            }
            PropertyKind::Padding => unreachable!(),
            PropertyKind::VariantId => unreachable!(),
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uint() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = Reader::new(content, End::None);
        let prop = IntProperty::new(Offset::new(0), ByteSize::U1, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xFE);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U1, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xBA);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U2, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xFEDC);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U2, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xBA98);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U3, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xFEDCBA);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U3, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xBA9876);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U4, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xFEDCBA98);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U4, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xBA987654);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U5, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xFEDCBA9876);
        let prop = IntProperty::new(Offset::new(2), ByteSize::U5, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0xBA98765432);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U6, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            0xFEDCBA987654
        );
        let prop = IntProperty::new(Offset::new(2), ByteSize::U6, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            0xBA9876543210
        );

        let prop = IntProperty::new(Offset::new(0), ByteSize::U7, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            0xFEDCBA98765432
        );
        let prop = IntProperty::new(Offset::new(2), ByteSize::U7, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            0xBA9876543210ff
        );

        let prop = IntProperty::new(Offset::new(0), ByteSize::U8, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            0xFEDCBA9876543210
        );
        let prop = IntProperty::new(Offset::new(1), ByteSize::U8, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            0xDCBA9876543210ff
        );
    }

    #[test]
    fn test_sint() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = Reader::new(content, End::None);
        let prop = SignedProperty::new(Offset::new(0), ByteSize::U1, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x02);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U1, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x46);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U2, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x0124);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U2, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x4568);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U3, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x012346);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U3, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x45678a);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U4, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x01234568);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U4, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x456789ac);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U5, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x012345678a);
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U5, None, None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -0x456789abce);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U6, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -0x0123456789ac
        );
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U6, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -0x456789abcdf0
        );

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U7, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -0x0123456789abce
        );
        let prop = SignedProperty::new(Offset::new(2), ByteSize::U7, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -0x456789abcdef01
        );

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U8, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -0x0123456789abcdf0
        );
        let prop = SignedProperty::new(Offset::new(1), ByteSize::U8, None, None);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -0x23456789abcdef01
        );
    }

    #[test]
    fn test_default_int() {
        let reader = Reader::new(vec![], End::None);
        let prop = IntProperty::new(Offset::new(0), ByteSize::U1, Some(50), None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 50);

        let prop = IntProperty::new(Offset::new(0), ByteSize::U2, Some(500), None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 500);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U1, Some(-50), None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -50);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U1, Some(50), None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 50);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U2, Some(500), None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 500);

        let prop = SignedProperty::new(Offset::new(0), ByteSize::U2, Some(-500), None);
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -500);
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
            fn get_data(&self, idx: ValueIdx, size: Option<Size>) -> Result<&[u8]> {
                let idx = idx.into_usize();
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
        let content = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x1A, 0x04, 0x20, 0xff];
        let reader = Reader::new(content, End::None);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())), // Size of the key
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0);

        let prop = IntProperty::new(
            Offset::new(2),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 2);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x01);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x0001);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x0102);

        let prop = IntProperty::new(
            Offset::new(0),
            ByteSize::U3, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x000102);

        let prop = IntProperty::new(
            Offset::new(2),
            ByteSize::U3, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x020304);

        let prop = IntProperty::new(
            Offset::new(4),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x04);

        let prop = IntProperty::new(
            Offset::new(4),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x0405);

        let prop = IntProperty::new(
            Offset::new(4),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x96);

        let prop = IntProperty::new(
            Offset::new(4),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x9697);

        let prop = IntProperty::new(
            Offset::new(6),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x9C);

        let prop = IntProperty::new(
            Offset::new(6),
            ByteSize::U8, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            0x9C9D9E9FA0A1A2A3
        );

        let prop = IntProperty::new(
            Offset::new(6),
            ByteSize::U8, // Size of the integer
            Some(20),
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 20);
    }

    #[test]
    fn test_deported_sint() {
        let value_store = Arc::new(mock::ValueStore::new());
        let content = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x1A, 0x04, 0x20, 0xff];
        let reader = Reader::new(content, End::None);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())), // Size of the key
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0);

        let prop = SignedProperty::new(
            Offset::new(2),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 2);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x01);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x0001);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x0102);

        let prop = SignedProperty::new(
            Offset::new(0),
            ByteSize::U3, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x000102);

        let prop = SignedProperty::new(
            Offset::new(2),
            ByteSize::U3, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x020304);

        let prop = SignedProperty::new(
            Offset::new(4),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x04);

        let prop = SignedProperty::new(
            Offset::new(4),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U1, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), 0x0405);

        let prop = SignedProperty::new(
            Offset::new(4),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -106_i64 //0x96_i8 overflowing
        );

        let prop = SignedProperty::new(
            Offset::new(4),
            ByteSize::U2, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -26985_i64 // 0x9697_i16 overflowing
        );

        let prop = SignedProperty::new(
            Offset::new(6),
            ByteSize::U1, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -100_i64 // 0x9C_i8 overflowing
        );

        let prop = SignedProperty::new(
            Offset::new(6),
            ByteSize::U8, // Size of the integer
            None,
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            -7161393424017284445_i64 // 0x9C9D9E9FA0A1A2A3 overflowing
        );

        let prop = SignedProperty::new(
            Offset::new(6),
            ByteSize::U8, // Size of the integer
            Some(-20),
            Some((ByteSize::U2, value_store.clone())),
        );
        assert_eq!(prop.create(&reader.as_sub_reader()).unwrap(), -20);
    }

    #[derive(Debug)]
    struct FakeArray {
        size: Option<Size>,
        base: BaseArray,
        base_len: u8,
        extend: Option<ValueIdx>,
    }

    impl FakeArray {
        fn new(
            size: Option<Size>,
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
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = Reader::new(content, End::None);
        let prop = ArrayProperty::new(Offset::new(0), Some(ByteSize::U1), 1, None, None);
        assert_eq!(
            FakeArray::new(Some(Size::new(0xFE)), BaseArray::new(&[0xDC]), 1, None),
            prop.create(&reader.as_sub_reader()).unwrap()
        );
        let prop = ArrayProperty::new(Offset::new(2), Some(ByteSize::U1), 1, None, None);
        assert_eq!(
            FakeArray::new(Some(Size::new(0xBA)), BaseArray::new(&[0x98]), 1, None),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );

        let prop = ArrayProperty::new(Offset::new(0), Some(ByteSize::U1), 2, None, None);
        assert_eq!(
            FakeArray::new(
                Some(Size::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA]),
                2,
                None
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );
        let prop = ArrayProperty::new(Offset::new(2), Some(ByteSize::U1), 2, None, None);
        assert_eq!(
            FakeArray::new(
                Some(Size::new(0xBA)),
                BaseArray::new(&[0x98, 0x76]),
                2,
                None
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );

        let prop = ArrayProperty::new(Offset::new(0), Some(ByteSize::U1), 3, None, None);
        assert_eq!(
            FakeArray::new(
                Some(Size::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA, 0x98]),
                3,
                None
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );
        let prop = ArrayProperty::new(Offset::new(2), Some(ByteSize::U1), 3, None, None);
        assert_eq!(
            FakeArray::new(
                Some(Size::new(0xBA)),
                BaseArray::new(&[0x98, 0x76, 0x54]),
                3,
                None
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );

        let prop = ArrayProperty::new(Offset::new(0), Some(ByteSize::U1), 8, None, None);
        assert_eq!(
            FakeArray::new(
                Some(Size::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff]),
                8,
                None
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );
    }

    mod fake {
        use super::*;
        use crate::reader::directory_pack::ValueStoreTrait;
        #[derive(Debug)]
        pub struct ValueStore {}

        impl ValueStoreTrait for ValueStore {
            fn get_data(&self, _id: ValueIdx, _size: Option<Size>) -> Result<&[u8]> {
                unreachable!()
            }
        }
    }

    #[test]
    fn test_deported_array() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = Reader::new(content, End::None);
        let value_store = Arc::new(fake::ValueStore {}) as Arc<dyn ValueStoreTrait>;
        let prop = ArrayProperty::new(
            Offset::new(0),
            Some(ByteSize::U1),
            0,
            Some((ByteSize::U1, Arc::clone(&value_store))),
            None,
        );
        assert_eq!(
            FakeArray::new(
                Some(Size::new(0xFE)),
                BaseArray::default(),
                0,
                Some(0xDC.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xBA)),
                BaseArray::default(),
                0,
                Some(0x98.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xFE)),
                BaseArray::default(),
                0,
                Some(0xDCBA.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xBA)),
                BaseArray::default(),
                0,
                Some(0x9876.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xFE)),
                BaseArray::new(&[0xDC]),
                1,
                Some(0xBA.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xBA)),
                BaseArray::new(&[0x98]),
                1,
                Some(0x76.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA, 0x98]),
                3,
                Some(0x76.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xBA)),
                BaseArray::new(&[0x98, 0x76, 0x54]),
                3,
                Some(0x32.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xFE)),
                BaseArray::new(&[0xDC, 0xBA, 0x98]),
                3,
                Some(0x765432.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
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
                Some(Size::new(0xBA)),
                BaseArray::new(&[0x98, 0x76, 0x54]),
                3,
                Some(0x3210ff.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );
    }

    #[test]
    fn default_array() {
        let reader = Reader::new(vec![], End::None);
        let value_store = Arc::new(fake::ValueStore {}) as Arc<dyn ValueStoreTrait>;

        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U1),
            3,
            None,
            Some((2, BaseArray::new(&[0x01, 0x02]), None)),
        );
        assert_eq!(
            FakeArray::new(Some(Size::new(2)), BaseArray::new(&[0x01, 0x02]), 3, None),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );

        let prop = ArrayProperty::new(
            Offset::new(2),
            Some(ByteSize::U3),
            3,
            Some((ByteSize::U2, Arc::clone(&value_store))),
            Some((2000, BaseArray::new(&[0x01, 0x02]), Some(300))),
        );
        assert_eq!(
            FakeArray::new(
                Some(Size::new(2000)),
                BaseArray::new(&[0x01, 0x02, 0x00]),
                3,
                Some(300.into())
            ),
            prop.create(&reader.as_sub_reader()).unwrap(),
        );
    }

    #[test]
    fn test_content() {
        let content = vec![0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10, 0xff];
        let reader = Reader::new(content, End::None);

        let prop = ContentProperty::new(Offset::new(0), None, ByteSize::U1, ByteSize::U3);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            ContentAddress::new(PackId::from(0xFE), ContentIdx::from(0xDCBA98))
        );

        let prop = ContentProperty::new(Offset::new(2), None, ByteSize::U1, ByteSize::U3);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            ContentAddress::new(PackId::from(0xBA), ContentIdx::from(0x987654))
        );

        let prop = ContentProperty::new(Offset::new(2), None, ByteSize::U1, ByteSize::U2);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            ContentAddress::new(PackId::from(0xBA), ContentIdx::from(0x9876))
        );

        let prop = ContentProperty::new(
            Offset::new(2),
            Some(PackId::from(0xFF)),
            ByteSize::U1,
            ByteSize::U3,
        );
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            ContentAddress::new(PackId::from(0xFF), ContentIdx::from(0xBA9876))
        );

        let prop = ContentProperty::new(
            Offset::new(2),
            Some(PackId::from(0xFF)),
            ByteSize::U1,
            ByteSize::U1,
        );
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            ContentAddress::new(PackId::from(0xFF), ContentIdx::from(0xBA))
        );

        let prop = ContentProperty::new(Offset::new(0), None, ByteSize::U2, ByteSize::U3);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            ContentAddress::new(PackId::from(0xFEDC), ContentIdx::from(0xBA9876))
        );

        let prop = ContentProperty::new(Offset::new(2), None, ByteSize::U2, ByteSize::U3);
        assert_eq!(
            prop.create(&reader.as_sub_reader()).unwrap(),
            ContentAddress::new(PackId::from(0xBA98), ContentIdx::from(0x765432))
        );
    }
}
