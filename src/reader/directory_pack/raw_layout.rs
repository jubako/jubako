use crate::bases::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeportedDefault {
    Value(u64),
    KeySize(ByteSize),
}

/// The kind of property definition as specified in the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyKind {
    Padding,
    ContentAddress(
        // The size of the pack_id
        ByteSize,
        // The size of the content_id
        ByteSize,
        // The default value of the pack_id
        Option<PackId>,
    ),
    UnsignedInt(
        // The size of the integer
        ByteSize,
        // The default value
        Option<u64>,
    ),
    SignedInt(
        // The size of the integer
        ByteSize,
        // The default value
        Option<i64>,
    ),
    DeportedUnsignedInt(
        // The size of the integer
        ByteSize,
        // The store in which search the integer
        ValueStoreIdx,
        // The default id value or the id size
        DeportedDefault,
    ),
    DeportedSignedInt(
        // The size of the integer
        ByteSize,
        // The store in which search the integer
        ValueStoreIdx,
        // The default id value or the id size
        DeportedDefault,
    ),
    Array(
        // The size of the array size.
        Option<ByteSize>,
        // The size of the fixed array
        u8,
        // Information on the deported array
        Option<(
            // The size of the id
            ByteSize,
            // The store in which search the deported array
            ValueStoreIdx,
        )>,
        // The default value
        Option<(u64, BaseArray, Option<u64>)>,
    ),
    VariantId,
}

/// The property definition as defined in the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProperty {
    pub size: usize, // The size of the value stored in the entry.
    pub name: Option<String>,
    pub kind: PropertyKind,
}

impl RawProperty {
    pub fn new(kind: PropertyKind, size: usize, name: Option<String>) -> Self {
        Self { size, kind, name }
    }

    pub fn is_variant_id(&self) -> bool {
        self.kind == PropertyKind::VariantId
    }
}

impl Producable for RawProperty {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let propinfo = flux.read_u8()?;
        let proptype =
            PropType::try_from(propinfo & 0xF0).map_err::<Error, _>(|e| format_error!(&e, flux))?;
        let propdata = propinfo & 0x0F;
        let (propsize, kind, name) = match proptype {
            PropType::Padding => (propdata as u16 + 1, PropertyKind::Padding, None),
            PropType::ContentAddress => {
                let pack_id_size =
                    ByteSize::try_from(((propdata & 0b0100) >> 2) as usize + 1).unwrap();
                let content_id_size = (propdata & 0b0011) as u16 + 1;
                let pack_id_default = if (propdata & 0b1000) != 0 {
                    Some((flux.read_usized(pack_id_size)? as u16).into())
                } else {
                    None
                };
                (
                    content_id_size
                        + if pack_id_default.is_some() {
                            0
                        } else {
                            pack_id_size as u16
                        },
                    PropertyKind::ContentAddress(
                        pack_id_size,
                        ByteSize::try_from(content_id_size as usize).unwrap(),
                        pack_id_default,
                    ),
                    Some(String::from_utf8(PString::produce(flux)?)?),
                )
            }
            PropType::UnsignedInt | PropType::SignedInt => {
                let default_value = (propdata & 0b1000) != 0;
                let int_size = ByteSize::try_from((propdata & 0x07) as usize + 1).unwrap();
                if default_value {
                    (
                        0,
                        if proptype == PropType::UnsignedInt {
                            PropertyKind::UnsignedInt(int_size, Some(flux.read_usized(int_size)?))
                        } else {
                            PropertyKind::SignedInt(int_size, Some(flux.read_isized(int_size)?))
                        },
                        Some(String::from_utf8(PString::produce(flux)?)?),
                    )
                } else {
                    (
                        (int_size as usize) as u16,
                        if proptype == PropType::UnsignedInt {
                            PropertyKind::UnsignedInt(int_size, None)
                        } else {
                            PropertyKind::SignedInt(int_size, None)
                        },
                        Some(String::from_utf8(PString::produce(flux)?)?),
                    )
                }
            }

            PropType::Array => {
                let default_value = (propdata & 0b1000) != 0;
                let size_size = propdata & 0x07;
                let size_size = if size_size != 0 {
                    Some(ByteSize::try_from(size_size as usize).unwrap())
                } else {
                    None
                };
                let complement = flux.read_u8()?;
                let fixed_array_size = complement & 0b00011111;
                let key_size = complement >> 5;
                let deported_info = if key_size != 0 {
                    Some((
                        ByteSize::try_from(key_size as usize).unwrap(),
                        ValueStoreIdx::from(flux.read_u8()?),
                    ))
                } else {
                    None
                };

                if default_value {
                    (
                        0,
                        {
                            let size = flux.read_usized(size_size.unwrap())?;
                            let fixed_data = BaseArray::new_from_flux(fixed_array_size, flux)?;
                            let key_id =
                                if key_size != 0 {
                                    Some(flux.read_usized(
                                        ByteSize::try_from(key_size as usize).unwrap(),
                                    )?)
                                } else {
                                    None
                                };
                            PropertyKind::Array(
                                size_size,
                                fixed_array_size,
                                deported_info,
                                Some((size, fixed_data, key_id)),
                            )
                        },
                        Some(String::from_utf8(PString::produce(flux)?)?),
                    )
                } else {
                    (
                        match size_size {
                            None => 0,
                            Some(s) => s as usize as u16,
                        } + fixed_array_size as u16
                            + key_size as u16,
                        PropertyKind::Array(size_size, fixed_array_size, deported_info, None),
                        Some(String::from_utf8(PString::produce(flux)?)?),
                    )
                }
            }
            PropType::VariantId => (
                1,
                PropertyKind::VariantId,
                Some(String::from_utf8(PString::produce(flux)?)?),
            ),
            PropType::DeportedUnsignedInt | PropType::DeportedSignedInt => {
                let default_value = (propdata & 0b1000) != 0;
                let int_size = ByteSize::try_from((propdata & 0x07) as usize + 1).unwrap();
                let key_id_size =
                    ByteSize::try_from((flux.read_u8()? & 0x07) as usize + 1).unwrap();
                let key_store_id = ValueStoreIdx::from(flux.read_u8()?);
                if default_value {
                    let key_value = flux.read_usized(key_id_size)?;
                    (
                        0,
                        if proptype == PropType::DeportedUnsignedInt {
                            PropertyKind::DeportedUnsignedInt(
                                int_size,
                                key_store_id,
                                DeportedDefault::Value(key_value),
                            )
                        } else {
                            PropertyKind::DeportedSignedInt(
                                int_size,
                                key_store_id,
                                DeportedDefault::Value(key_value),
                            )
                        },
                        Some(String::from_utf8(PString::produce(flux)?)?),
                    )
                } else {
                    (
                        (key_id_size as usize) as u16,
                        if proptype == PropType::DeportedUnsignedInt {
                            PropertyKind::DeportedUnsignedInt(
                                int_size,
                                key_store_id,
                                DeportedDefault::KeySize(key_id_size),
                            )
                        } else {
                            PropertyKind::DeportedSignedInt(
                                int_size,
                                key_store_id,
                                DeportedDefault::KeySize(key_id_size),
                            )
                        },
                        Some(String::from_utf8(PString::produce(flux)?)?),
                    )
                }
            }
        };
        Ok(Self {
            size: propsize as usize,
            kind,
            name,
        })
    }
}

pub struct RawLayout(Vec<RawProperty>);

impl std::ops::Deref for RawLayout {
    type Target = Vec<RawProperty>;
    fn deref(&self) -> &Vec<RawProperty> {
        &self.0
    }
}

impl std::ops::DerefMut for RawLayout {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Producable for RawLayout {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let property_count: PropertyCount = Count::<u8>::produce(flux)?.into();
        let mut properties = Vec::with_capacity(property_count.into_usize());
        for _ in property_count {
            let property = RawProperty::produce(flux)?;
            properties.push(property);
        }
        Ok(Self(properties))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(&[0b1000_0000, 1, b'a'] => RawProperty{size:1, kind:PropertyKind::VariantId, name: Some(String::from("a")) })]
    #[test_case(&[0b0000_0000] => RawProperty{size:1, kind:PropertyKind::Padding, name: None })]
    #[test_case(&[0b0000_0111] => RawProperty{size:8, kind:PropertyKind::Padding, name: None })]
    #[test_case(&[0b0000_1111] => RawProperty{size:16, kind:PropertyKind::Padding, name: None })]
    // ContentAddress
    #[test_case(&[0b0001_0000, 1, b'a'] => RawProperty{size:2, kind:PropertyKind::ContentAddress(ByteSize::U1, ByteSize::U1, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0001_0001, 1, b'a'] => RawProperty{size:3, kind:PropertyKind::ContentAddress(ByteSize::U1, ByteSize::U2, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0001_0110, 1, b'a'] => RawProperty{size:5, kind:PropertyKind::ContentAddress(ByteSize::U2, ByteSize::U3, None), name: Some(String::from("a")) })]
    // ContentAddress with default pack_id
    #[test_case(&[0b0001_1000, 0x01, 1, b'a'] => RawProperty{size:1, kind:PropertyKind::ContentAddress(ByteSize::U1, ByteSize::U1, Some(1.into())), name: Some(String::from("a")) })]
    #[test_case(&[0b0001_1001, 0x01, 1, b'a'] => RawProperty{size:2, kind:PropertyKind::ContentAddress(ByteSize::U1, ByteSize::U2, Some(1.into())), name: Some(String::from("a")) })]
    #[test_case(&[0b0001_1110, 0x01, 0x02, 1, b'a'] => RawProperty{size:3, kind:PropertyKind::ContentAddress(ByteSize::U2, ByteSize::U3, Some(0x0201.into())), name: Some(String::from("a")) })]
    // Plain integer
    #[test_case(&[0b0010_0000, 1, b'a'] => RawProperty{size:1, kind:PropertyKind::UnsignedInt(ByteSize::U1, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0010_0010, 1, b'a'] => RawProperty{size:3, kind:PropertyKind::UnsignedInt(ByteSize::U3, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0010_0111, 1, b'a'] => RawProperty{size:8, kind:PropertyKind::UnsignedInt(ByteSize::U8, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0011_0000, 1, b'a'] => RawProperty{size:1, kind:PropertyKind::SignedInt(ByteSize::U1, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0011_0010, 1, b'a'] => RawProperty{size:3, kind:PropertyKind::SignedInt(ByteSize::U3, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0011_0111, 1, b'a'] => RawProperty{size:8, kind:PropertyKind::SignedInt(ByteSize::U8, None), name: Some(String::from("a")) })]
    // Plain integer with default value
    #[test_case(&[0b0010_1000, 0xff, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::UnsignedInt(ByteSize::U1, Some(0xff)), name: Some(String::from("a")) })]
    #[test_case(&[0b0010_1010, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::UnsignedInt(ByteSize::U3, Some(0x010203)), name: Some(String::from("a")) })]
    #[test_case(&[0b0010_1111, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::UnsignedInt(ByteSize::U8, Some(0x0102030405060708)), name: Some(String::from("a")) })]
    #[test_case(&[0b0011_1000, 0xff, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::SignedInt(ByteSize::U1, Some(-1_i64)), name: Some(String::from("a")) })]
    #[test_case(&[0b0011_1010, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::SignedInt(ByteSize::U3, Some(0x010203_i64)), name: Some(String::from("a")) })]
    #[test_case(&[0b0011_1111, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::SignedInt(ByteSize::U8, Some(0x0102030405060708_i64)), name: Some(String::from("a")) })]
    // Deported integer
    #[test_case(&[0b1010_0000, 0b0000_0000, 0xff, 1, b'a'] => RawProperty{size:1, kind:PropertyKind::DeportedUnsignedInt(ByteSize::U1, 0xff.into(), DeportedDefault::KeySize(ByteSize::U1)), name: Some(String::from("a")) })]
    #[test_case(&[0b1010_0010, 0b0000_0001, 0xff, 1, b'a'] => RawProperty{size:2, kind:PropertyKind::DeportedUnsignedInt(ByteSize::U3, 0xff.into(), DeportedDefault::KeySize(ByteSize::U2)), name: Some(String::from("a")) })]
    #[test_case(&[0b1010_0111, 0b0000_0111, 0xff, 1, b'a'] => RawProperty{size:8, kind:PropertyKind::DeportedUnsignedInt(ByteSize::U8, 0xff.into(), DeportedDefault::KeySize(ByteSize::U8)), name: Some(String::from("a")) })]
    #[test_case(&[0b1011_0000, 0b0000_0111, 0xff, 1, b'a'] => RawProperty{size:8, kind:PropertyKind::DeportedSignedInt(ByteSize::U1, 0xff.into(), DeportedDefault::KeySize(ByteSize::U8)), name: Some(String::from("a")) })]
    #[test_case(&[0b1011_0010, 0b0000_0001, 0xff, 1, b'a'] => RawProperty{size:2, kind:PropertyKind::DeportedSignedInt(ByteSize::U3, 0xff.into(), DeportedDefault::KeySize(ByteSize::U2)), name: Some(String::from("a")) })]
    #[test_case(&[0b1011_0111, 0b0000_0010, 0xff, 1, b'a'] => RawProperty{size:3, kind:PropertyKind::DeportedSignedInt(ByteSize::U8, 0xff.into(), DeportedDefault::KeySize(ByteSize::U3)), name: Some(String::from("a")) })]
    // Deported integer with default index
    #[test_case(&[0b1010_1000, 0b0000_0000, 0xff, 0xff, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::DeportedUnsignedInt(ByteSize::U1, 0xff.into(), DeportedDefault::Value(0xff_u64)), name: Some(String::from("a")) })]
    #[test_case(&[0b1010_1010, 0b0000_0001, 0xff, 0xfe, 0xff, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::DeportedUnsignedInt(ByteSize::U3, 0xff.into(), DeportedDefault::Value(0xfffe_u64)), name: Some(String::from("a")) })]
    #[test_case(&[0b1010_1111, 0b0000_0010, 0xff, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::DeportedUnsignedInt(ByteSize::U8, 0xff.into(), DeportedDefault::Value(0x010203_u64)), name: Some(String::from("a")) })]
    #[test_case(&[0b1011_1000, 0b0000_0011, 0xff, 0xff, 0xff, 0xff, 0xff, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::DeportedSignedInt(ByteSize::U1, 0xff.into(), DeportedDefault::Value(0xffffffff_u64)), name: Some(String::from("a")) })]
    #[test_case(&[0b1011_1010, 0b0000_0001, 0xff, 0xff, 0xff, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::DeportedSignedInt(ByteSize::U3, 0xff.into(), DeportedDefault::Value(0xffff_u64)), name: Some(String::from("a")) })]
    #[test_case(&[0b1011_1111, 0b0000_0010, 0xff, 0xff, 0xff, 0xff, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::DeportedSignedInt(ByteSize::U8, 0xff.into(), DeportedDefault::Value(0xffffff_u64)), name: Some(String::from("a")) })]
    // Char[] without deported part :
    #[test_case(&[0b0101_0001, 0b000_00000, 1, b'a'] => RawProperty{size:1+0+0, kind:PropertyKind::Array(Some(ByteSize::U1), 0, None, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_0001, 0b000_00001, 1, b'a'] => RawProperty{size:1+1+0, kind:PropertyKind::Array(Some(ByteSize::U1), 1, None, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_0011, 0b000_00101, 1, b'a'] => RawProperty{size:3+5+0, kind:PropertyKind::Array(Some(ByteSize::U3), 5, None, None), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_0111, 0b000_11111, 1, b'a'] => RawProperty{size:7+31+0, kind:PropertyKind::Array(Some(ByteSize::U7), 31, None, None), name: Some(String::from("a")) })]
    // Char[] without deported part and with default value:
    #[test_case(&[0b0101_1001, 0b000_00000, 0x00, 1, b'a'] => RawProperty{size:0, kind:PropertyKind::Array(Some(ByteSize::U1), 0, None, Some((0, BaseArray::default(), None))), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_1001, 0b000_00001, 0x01, b'a', 1, b'a'] => RawProperty{size:0, kind:PropertyKind::Array(Some(ByteSize::U1), 1, None, Some((1, BaseArray::new(b"a"), None))), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_1011, 0b000_00101, 0x04, 0x00, 0x00, b'a', b'b', b'c', b'd', b'\0', 1, b'a'] => RawProperty{size:0, kind:PropertyKind::Array(Some(ByteSize::U3), 5, None, Some((4, BaseArray::new(b"abcd"), None))), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_1001, 0b000_11111,
      0x1A,
      b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z', 0x00, 0x00, 0x00, 0x00, 0x00, 1, b'a' ] =>
      RawProperty{size:0, kind:PropertyKind::Array(Some(ByteSize::U1), 31, None, Some((26, BaseArray::new(b"abcdefghijklmnopqrstuvwxyz"), None))), name: Some(String::from("a")) })]
    // Char[] with deported part :
    #[test_case(&[0b0101_0001, 0b001_00000, 0x0F, 1, b'a'] => RawProperty{size:1+0+1, kind:PropertyKind::Array(Some(ByteSize::U1), 0, Some((ByteSize::U1, ValueStoreIdx::from(0x0F))), None), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_0001, 0b010_00001, 0x0F, 1, b'a'] => RawProperty{size:1+1+2, kind:PropertyKind::Array(Some(ByteSize::U1), 1, Some((ByteSize::U2, ValueStoreIdx::from(0x0F))), None), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_0011, 0b100_00101, 0x0F, 1, b'a'] => RawProperty{size:3+5+4, kind:PropertyKind::Array(Some(ByteSize::U3), 5, Some((ByteSize::U4, ValueStoreIdx::from(0x0F))), None), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_0111, 0b100_11111, 0x0F, 1, b'a'] => RawProperty{size:7+31+4, kind:PropertyKind::Array(Some(ByteSize::U7), 31, Some((ByteSize::U4, ValueStoreIdx::from(0x0F))), None), name: Some(String::from("a")) })]
    // Char[] without deported part and with default value:
    #[test_case(&[0b0101_1001, 0b001_00000, 0x0F, 0x00, 0x50, 1, b'a']
      => RawProperty{size:0, kind:PropertyKind::Array(Some(ByteSize::U1), 0, Some((ByteSize::U1, ValueStoreIdx::from(0x0F))), Some((0, BaseArray::default(), Some(0x50)))), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_1001, 0b010_00001, 0x0F, 0x10, b'a', 0x50, 0x00, 1, b'a']
      => RawProperty{size:0, kind:PropertyKind::Array(Some(ByteSize::U1), 1, Some((ByteSize::U2, ValueStoreIdx::from(0x0F))), Some((16, BaseArray::new(b"a"), Some(0x50)))), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_1011, 0b100_00101, 0x0F, 0x04, 0x00, 0x00, b'a', b'b', b'c', b'd', b'\0', 0xfc, 0xfd, 0xfe, 0xff, 1, b'a']
      => RawProperty{size:0, kind:PropertyKind::Array(Some(ByteSize::U3), 5, Some((ByteSize::U4, ValueStoreIdx::from(0x0F))), Some((4, BaseArray::new(b"abcd"), Some(0xfffefdfc)))), name: Some(String::from("a")) })]
    #[test_case(&[0b0101_1111, 0b100_11111, 0x0F,
      0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
      b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z', 0x00, 0x00, 0x00, 0x00, 0x00,
      0xfc, 0xfd, 0xfe, 0xff, 1, b'a' ]
       => RawProperty{size:0, kind:PropertyKind::Array(Some(ByteSize::U7), 31, Some((ByteSize::U4, ValueStoreIdx::from(0x0F))), Some((0x01020304050607, BaseArray::new(b"abcdefghijklmnopqrstuvwxyz"), Some(0xfffefdfc)))), name: Some(String::from("a")) })]

    fn test_rawproperty(source: &[u8]) -> RawProperty {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        RawProperty::produce(&mut flux).unwrap()
    }
}
