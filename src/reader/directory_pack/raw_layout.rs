use crate::bases::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub(crate) enum DeportedDefault {
    Value(u64),
    KeySize(ByteSize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub(crate) struct DeportedInfo {
    // The size of the id
    pub id_size: ByteSize,
    // The store in which search the deported array
    pub value_store_idx: ValueStoreIdx,
}

#[cfg(feature = "explorable")]
impl graphex::Display for DeportedInfo {
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        out.field("value_store_idx", &self.value_store_idx.into_u64())?;
        out.field("id_size", &(self.id_size as usize))
    }
}

/// The kind of property definition as specified in the store.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub(crate) enum PropertyKind {
    Padding,
    ContentAddress {
        pack_id_size: ByteSize,
        content_id_size: ByteSize,
        default_pack_id: Option<PackId>,
    },
    UnsignedInt {
        int_size: ByteSize,
        default: Option<u64>,
    },
    SignedInt {
        int_size: ByteSize,
        default: Option<i64>,
    },
    DeportedUnsignedInt {
        int_size: ByteSize,
        value_store_idx: ValueStoreIdx,
        id: DeportedDefault,
    },
    DeportedSignedInt {
        int_size: ByteSize,
        value_store_idx: ValueStoreIdx,
        id: DeportedDefault,
    },
    Array {
        array_len_size: Option<ByteSize>,
        fixed_array_len: u8,
        deported_info: Option<DeportedInfo>,
        default: Option<(ASize, BaseArray, Option<u64>)>,
    },
    VariantId,
}

/// The property definition as defined in the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawProperty {
    pub size: usize, // The size of the value stored in the entry.
    pub name: SmallString,
    pub kind: PropertyKind,
}

impl RawProperty {
    #[cfg(test)]
    pub fn new(kind: PropertyKind, size: usize, name: impl Into<SmallString>) -> Self {
        Self {
            size,
            kind,
            name: name.into(),
        }
    }

    pub fn is_variant_id(&self) -> bool {
        self.kind == PropertyKind::VariantId
    }
}

impl Parsable for RawProperty {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let propinfo = parser.read_u8()?;
        let proptype = PropType::try_from(propinfo & 0xF0)?;
        let propdata = propinfo & 0x0F;
        let (propsize, kind, name) = match proptype {
            PropType::Padding => (
                propdata as u16 + 1,
                PropertyKind::Padding,
                SmallString::default(),
            ),
            PropType::ContentAddress => {
                let pack_id_size =
                    ByteSize::try_from(((propdata & 0b0100) >> 2) as usize + 1).unwrap();
                let content_id_size = (propdata & 0b0011) as u16 + 1;
                let default_pack_id = if (propdata & 0b1000) != 0 {
                    Some((parser.read_usized(pack_id_size)? as u16).into())
                } else {
                    None
                };
                (
                    content_id_size
                        + if default_pack_id.is_some() {
                            0
                        } else {
                            pack_id_size as u16
                        },
                    PropertyKind::ContentAddress {
                        pack_id_size,
                        content_id_size: ByteSize::try_from(content_id_size as usize).unwrap(),
                        default_pack_id,
                    },
                    PString::parse(parser)?,
                )
            }
            PropType::UnsignedInt | PropType::SignedInt => {
                let default_value = (propdata & 0b1000) != 0;
                let int_size = ByteSize::try_from((propdata & 0x07) as usize + 1).unwrap();
                if default_value {
                    (
                        0,
                        if proptype == PropType::UnsignedInt {
                            PropertyKind::UnsignedInt {
                                int_size,
                                default: Some(parser.read_usized(int_size)?),
                            }
                        } else {
                            PropertyKind::SignedInt {
                                int_size,
                                default: Some(parser.read_isized(int_size)?),
                            }
                        },
                        PString::parse(parser)?,
                    )
                } else {
                    (
                        (int_size as usize) as u16,
                        if proptype == PropType::UnsignedInt {
                            PropertyKind::UnsignedInt {
                                int_size,
                                default: None,
                            }
                        } else {
                            PropertyKind::SignedInt {
                                int_size,
                                default: None,
                            }
                        },
                        PString::parse(parser)?,
                    )
                }
            }

            PropType::Array => {
                let default_value = (propdata & 0b1000) != 0;
                let array_len_size = propdata & 0b0011;
                let array_len_size = if array_len_size != 0 {
                    Some(ByteSize::try_from(array_len_size as usize).unwrap())
                } else {
                    None
                };
                let complement = parser.read_u8()?;
                let fixed_array_len = complement & 0b00011111;
                let key_size = complement >> 5;
                let deported_info = if key_size != 0 {
                    Some(DeportedInfo {
                        id_size: ByteSize::try_from(key_size as usize).unwrap(),
                        value_store_idx: ValueStoreIdx::from(parser.read_u8()?),
                    })
                } else {
                    None
                };

                if default_value {
                    (
                        0,
                        {
                            let size = parser.read_usized(array_len_size.unwrap())?;
                            let fixed_data = BaseArray::parse(fixed_array_len, parser)?;
                            let key_id =
                                if key_size != 0 {
                                    Some(parser.read_usized(
                                        ByteSize::try_from(key_size as usize).unwrap(),
                                    )?)
                                } else {
                                    None
                                };
                            PropertyKind::Array {
                                array_len_size,
                                fixed_array_len,
                                deported_info,
                                default: Some((ASize::new(size as usize), fixed_data, key_id)),
                            }
                        },
                        PString::parse(parser)?,
                    )
                } else {
                    (
                        match array_len_size {
                            None => 0,
                            Some(s) => s as usize as u16,
                        } + fixed_array_len as u16
                            + key_size as u16,
                        PropertyKind::Array {
                            array_len_size,
                            fixed_array_len,
                            deported_info,
                            default: None,
                        },
                        PString::parse(parser)?,
                    )
                }
            }
            PropType::VariantId => (1, PropertyKind::VariantId, PString::parse(parser)?),
            PropType::DeportedUnsignedInt | PropType::DeportedSignedInt => {
                let default_value = (propdata & 0b1000) != 0;
                let int_size = ByteSize::try_from((propdata & 0x07) as usize + 1).unwrap();
                let key_id_size =
                    ByteSize::try_from((parser.read_u8()? & 0x07) as usize + 1).unwrap();
                let value_store_idx = ValueStoreIdx::from(parser.read_u8()?);
                if default_value {
                    let key_value = parser.read_usized(key_id_size)?;
                    (
                        0,
                        if proptype == PropType::DeportedUnsignedInt {
                            PropertyKind::DeportedUnsignedInt {
                                int_size,
                                value_store_idx,
                                id: DeportedDefault::Value(key_value),
                            }
                        } else {
                            PropertyKind::DeportedSignedInt {
                                int_size,
                                value_store_idx,
                                id: DeportedDefault::Value(key_value),
                            }
                        },
                        PString::parse(parser)?,
                    )
                } else {
                    (
                        (key_id_size as usize) as u16,
                        if proptype == PropType::DeportedUnsignedInt {
                            PropertyKind::DeportedUnsignedInt {
                                int_size,
                                value_store_idx,
                                id: DeportedDefault::KeySize(key_id_size),
                            }
                        } else {
                            PropertyKind::DeportedSignedInt {
                                int_size,
                                value_store_idx,
                                id: DeportedDefault::KeySize(key_id_size),
                            }
                        },
                        PString::parse(parser)?,
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

pub(super) struct RawLayout(Vec<RawProperty>);

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

impl Parsable for RawLayout {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let property_count: PropertyCount = Count::<u8>::parse(parser)?.into();
        let mut properties = Vec::with_capacity(property_count.into_usize());
        for _ in property_count {
            let property = RawProperty::parse(parser)?;
            properties.push(property);
        }
        Ok(Self(properties))
    }
}

#[cfg(test)]
#[allow(clippy::identity_op)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(&[0b1000_0000, 1, b'a'] => RawProperty::new(PropertyKind::VariantId, 1, "a"))]
    #[test_case(&[0b0000_0000] => RawProperty::new(PropertyKind::Padding, 1, ""))]
    #[test_case(&[0b0000_0111] => RawProperty::new(PropertyKind::Padding, 8, ""))]
    #[test_case(&[0b0000_1111] => RawProperty::new(PropertyKind::Padding, 16, ""))]
    // ContentAddress
    #[test_case(&[0b0001_0000, 1, b'a'] => RawProperty::new(PropertyKind::ContentAddress{pack_id_size: ByteSize::U1, content_id_size: ByteSize::U1, default_pack_id: None}, 2, "a"))]
    #[test_case(&[0b0001_0001, 1, b'a'] => RawProperty::new(PropertyKind::ContentAddress{pack_id_size: ByteSize::U1, content_id_size: ByteSize::U2, default_pack_id: None}, 3, "a"))]
    #[test_case(&[0b0001_0110, 1, b'a'] => RawProperty::new(PropertyKind::ContentAddress{pack_id_size: ByteSize::U2, content_id_size: ByteSize::U3, default_pack_id: None}, 5, "a"))]
    // ContentAddress with default pack_id
    #[test_case(&[0b0001_1000, 0x01, 1, b'a'] => RawProperty::new(PropertyKind::ContentAddress{pack_id_size: ByteSize::U1, content_id_size: ByteSize::U1, default_pack_id: Some(1.into())},1, "a"))]
    #[test_case(&[0b0001_1001, 0x01, 1, b'a'] => RawProperty::new(PropertyKind::ContentAddress{pack_id_size: ByteSize::U1, content_id_size: ByteSize::U2, default_pack_id: Some(1.into())}, 2, "a"))]
    #[test_case(&[0b0001_1110, 0x01, 0x02, 1, b'a'] => RawProperty::new(PropertyKind::ContentAddress{pack_id_size: ByteSize::U2, content_id_size: ByteSize::U3, default_pack_id: Some(0x0201.into())}, 3, "a"))]
    // Plain integer
    #[test_case(&[0b0010_0000, 1, b'a'] => RawProperty::new(PropertyKind::UnsignedInt{int_size: ByteSize::U1, default: None}, 1, "a"))]
    #[test_case(&[0b0010_0010, 1, b'a'] => RawProperty::new(PropertyKind::UnsignedInt{int_size: ByteSize::U3, default: None}, 3, "a"))]
    #[test_case(&[0b0010_0111, 1, b'a'] => RawProperty::new(PropertyKind::UnsignedInt{int_size: ByteSize::U8, default: None}, 8, "a"))]
    #[test_case(&[0b0011_0000, 1, b'a'] => RawProperty::new(PropertyKind::SignedInt{int_size: ByteSize::U1, default: None}, 1, "a"))]
    #[test_case(&[0b0011_0010, 1, b'a'] => RawProperty::new(PropertyKind::SignedInt{int_size: ByteSize::U3, default: None}, 3, "a"))]
    #[test_case(&[0b0011_0111, 1, b'a'] => RawProperty::new(PropertyKind::SignedInt{int_size: ByteSize::U8, default: None}, 8, "a"))]
    // Plain integer with default value
    #[test_case(&[0b0010_1000, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::UnsignedInt{int_size: ByteSize::U1, default: Some(0xff)}, 0, "a"))]
    #[test_case(&[0b0010_1010, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty::new(PropertyKind::UnsignedInt{int_size: ByteSize::U3, default: Some(0x010203)}, 0, "a"))]
    #[test_case(&[0b0010_1111, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty::new(PropertyKind::UnsignedInt{int_size: ByteSize::U8, default: Some(0x0102030405060708)}, 0, "a"))]
    #[test_case(&[0b0011_1000, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::SignedInt{int_size: ByteSize::U1, default: Some(-1_i64)}, 0, "a"))]
    #[test_case(&[0b0011_1010, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty::new(PropertyKind::SignedInt{int_size: ByteSize::U3, default: Some(0x010203_i64)}, 0, "a"))]
    #[test_case(&[0b0011_1111, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty::new(PropertyKind::SignedInt{int_size: ByteSize::U8, default: Some(0x0102030405060708_i64)}, 0, "a"))]
    // Deported integer
    #[test_case(&[0b1010_0000, 0b0000_0000, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedUnsignedInt{int_size: ByteSize::U1, value_store_idx: 0xff.into(), id: DeportedDefault::KeySize(ByteSize::U1)}, 1, "a"))]
    #[test_case(&[0b1010_0010, 0b0000_0001, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedUnsignedInt{int_size: ByteSize::U3, value_store_idx: 0xff.into(), id: DeportedDefault::KeySize(ByteSize::U2)}, 2, "a"))]
    #[test_case(&[0b1010_0111, 0b0000_0111, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedUnsignedInt{int_size: ByteSize::U8, value_store_idx: 0xff.into(), id: DeportedDefault::KeySize(ByteSize::U8)}, 8, "a"))]
    #[test_case(&[0b1011_0000, 0b0000_0111, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedSignedInt{int_size: ByteSize::U1, value_store_idx: 0xff.into(), id: DeportedDefault::KeySize(ByteSize::U8)}, 8, "a"))]
    #[test_case(&[0b1011_0010, 0b0000_0001, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedSignedInt{int_size: ByteSize::U3, value_store_idx: 0xff.into(), id: DeportedDefault::KeySize(ByteSize::U2)}, 2, "a"))]
    #[test_case(&[0b1011_0111, 0b0000_0010, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedSignedInt{int_size: ByteSize::U8, value_store_idx: 0xff.into(), id: DeportedDefault::KeySize(ByteSize::U3)}, 3, "a"))]
    // Deported integer with default index
    #[test_case(&[0b1010_1000, 0b0000_0000, 0xff, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedUnsignedInt{int_size: ByteSize::U1, value_store_idx: 0xff.into(), id: DeportedDefault::Value(0xff_u64)}, 0, "a"))]
    #[test_case(&[0b1010_1010, 0b0000_0001, 0xff, 0xfe, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedUnsignedInt{int_size: ByteSize::U3, value_store_idx: 0xff.into(), id: DeportedDefault::Value(0xfffe_u64)}, 0, "a"))]
    #[test_case(&[0b1010_1111, 0b0000_0010, 0xff, 0x03, 0x02, 0x01, 1, b'a'] => RawProperty::new(PropertyKind::DeportedUnsignedInt{int_size: ByteSize::U8, value_store_idx: 0xff.into(), id: DeportedDefault::Value(0x010203_u64)}, 0, "a"))]
    #[test_case(&[0b1011_1000, 0b0000_0011, 0xff, 0xff, 0xff, 0xff, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedSignedInt{int_size: ByteSize::U1, value_store_idx: 0xff.into(), id: DeportedDefault::Value(0xffffffff_u64)}, 0, "a"))]
    #[test_case(&[0b1011_1010, 0b0000_0001, 0xff, 0xff, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedSignedInt{int_size: ByteSize::U3, value_store_idx: 0xff.into(), id: DeportedDefault::Value(0xffff_u64)}, 0, "a"))]
    #[test_case(&[0b1011_1111, 0b0000_0010, 0xff, 0xff, 0xff, 0xff, 1, b'a'] => RawProperty::new(PropertyKind::DeportedSignedInt{int_size: ByteSize::U8, value_store_idx: 0xff.into(), id: DeportedDefault::Value(0xffffff_u64)},0 , "a"))]
    // Char[] without deported part :
    #[test_case(&[0b0101_0001, 0b000_00000, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 0, deported_info: None, default: None}, 1+0+0, "a"))]
    #[test_case(&[0b0101_0001, 0b000_00001, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 1, deported_info: None, default: None}, 1+1+0, "a"))]
    #[test_case(&[0b0101_0011, 0b000_00101, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U3), fixed_array_len: 5, deported_info: None, default: None}, 3+5+0, "a"))]
    #[test_case(&[0b0101_0011, 0b000_11111, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U3), fixed_array_len: 31, deported_info: None, default: None}, 3+31+0, "a"))]
    // Char[] without deported part and with default value:
    #[test_case(&[0b0101_1001, 0b000_00000, 0x00, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 0, deported_info: None, default: Some((0.into(), BaseArray::default(), None))}, 0, "a"))]
    #[test_case(&[0b0101_1001, 0b000_00001, 0x01, b'a', 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 1, deported_info: None, default: Some((1.into(), BaseArray::new(b"a"), None))}, 0, "a"))]
    #[test_case(&[0b0101_1011, 0b000_00101, 0x04, 0x00, 0x00, b'a', b'b', b'c', b'd', b'\0', 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U3), fixed_array_len: 5, deported_info: None, default: Some((4.into(), BaseArray::new(b"abcd"), None))}, 0, "a"))]
    #[test_case(&[0b0101_1001, 0b000_11111,
      0x1A,
      b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z', 0x00, 0x00, 0x00, 0x00, 0x00, 1, b'a' ] =>
      RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 31, deported_info: None, default: Some((26.into(), BaseArray::new(b"abcdefghijklmnopqrstuvwxyz"), None))}, 0, "a"))]
    // Char[] with deported part :
    #[test_case(&[0b0101_0001, 0b001_00000, 0x0F, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 0, deported_info: Some(DeportedInfo{ id_size: ByteSize::U1, value_store_idx: ValueStoreIdx::from(0x0F)}), default: None}, 1+0+1, "a"))]
    #[test_case(&[0b0101_0001, 0b010_00001, 0x0F, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 1, deported_info: Some(DeportedInfo{ id_size: ByteSize::U2, value_store_idx: ValueStoreIdx::from(0x0F)}), default: None}, 1+1+2, "a"))]
    #[test_case(&[0b0101_0011, 0b100_00101, 0x0F, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U3), fixed_array_len: 5, deported_info: Some(DeportedInfo{ id_size: ByteSize::U4, value_store_idx: ValueStoreIdx::from(0x0F)}), default: None}, 3+5+4, "a"))]
    #[test_case(&[0b0101_0011, 0b100_11111, 0x0F, 1, b'a'] => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U3), fixed_array_len: 31, deported_info: Some(DeportedInfo{ id_size: ByteSize::U4, value_store_idx: ValueStoreIdx::from(0x0F)}), default: None}, 3+31+4, "a"))]
    // Char[] without deported part and with default value:
    #[test_case(&[0b0101_1001, 0b001_00000, 0x0F, 0x00, 0x50, 1, b'a']
      => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 0, deported_info: Some(DeportedInfo{ id_size: ByteSize::U1, value_store_idx: ValueStoreIdx::from(0x0F)}), default: Some((0.into(), BaseArray::default(), Some(0x50)))}, 0, "a"))]
    #[test_case(&[0b0101_1001, 0b010_00001, 0x0F, 0x10, b'a', 0x50, 0x00, 1, b'a']
      => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U1), fixed_array_len: 1, deported_info: Some(DeportedInfo{ id_size: ByteSize::U2, value_store_idx: ValueStoreIdx::from(0x0F)}), default: Some((16.into(), BaseArray::new(b"a"), Some(0x50)))}, 0, "a"))]
    #[test_case(&[0b0101_1011, 0b100_00101, 0x0F, 0x04, 0x00, 0x00, b'a', b'b', b'c', b'd', b'\0', 0xfc, 0xfd, 0xfe, 0xff, 1, b'a']
      => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U3), fixed_array_len: 5, deported_info: Some(DeportedInfo{ id_size: ByteSize::U4, value_store_idx: ValueStoreIdx::from(0x0F)}), default: Some((4.into(), BaseArray::new(b"abcd"), Some(0xfffefdfc)))}, 0, "a"))]
    #[test_case(&[0b0101_1011, 0b100_11111, 0x0F,
      0x03, 0x02, 0x01,
      b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z', 0x00, 0x00, 0x00, 0x00, 0x00,
      0xfc, 0xfd, 0xfe, 0xff, 1, b'a' ]
       => RawProperty::new(PropertyKind::Array{array_len_size: Some(ByteSize::U3), fixed_array_len: 31, deported_info: Some(DeportedInfo{ id_size: ByteSize::U4, value_store_idx: ValueStoreIdx::from(0x0F)}), default: Some((0x010203.into(), BaseArray::new(b"abcdefghijklmnopqrstuvwxyz"), Some(0xfffefdfc)))}, 0, "a"))]

    fn test_rawproperty(source: &[u8]) -> RawProperty {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let size = content.len();
        let reader = CheckReader::from(content);
        reader
            .parse_in::<RawProperty>(Offset::zero(), size.into())
            .unwrap()
    }
}
