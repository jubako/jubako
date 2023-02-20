use crate::bases::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeportedDefault {
    Value(u64),
    KeySize(ByteSize),
}

/// The kind of property definition as specified in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    Padding,
    ContentAddress(
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
        Option<(u64, [u8; 31], Option<u64>)>,
    ),
    VariantId,
}

/// The property definition as defined in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawProperty {
    pub size: usize, // The size of the value stored in the entry.
    pub kind: PropertyKind,
}

impl RawProperty {
    pub fn new(kind: PropertyKind, size: usize) -> Self {
        Self { size, kind }
    }

    pub fn is_variant_id(&self) -> bool {
        self.kind == PropertyKind::VariantId
    }
}

impl Producable for RawProperty {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let propinfo = flux.read_u8()?;
        let proptype = propinfo >> 4;
        let propdata = propinfo & 0x0F;
        let (propsize, kind) = match proptype {
            0b0000 => (propdata as u16 + 1, PropertyKind::Padding),
            0b0001 => {
                let content_id_size = (propdata & 0b0011) as u16 + 1;
                let pack_id_default = if (propdata & 0b0100) == 0 {
                    Some(flux.read_u8()?.into())
                } else {
                    None
                };
                (
                    content_id_size + if pack_id_default.is_some() { 0 } else { 1 },
                    PropertyKind::ContentAddress(
                        ByteSize::try_from(content_id_size as usize).unwrap(),
                        pack_id_default,
                    ),
                )
            }
            0b0010 | 0b0011 => {
                let default_value = (propdata & 0b1000) != 0;
                let int_size = ByteSize::try_from((propdata & 0x07) as usize + 1).unwrap();
                if default_value {
                    (
                        0,
                        if proptype == 0b0010 {
                            PropertyKind::UnsignedInt(int_size, Some(flux.read_usized(int_size)?))
                        } else {
                            PropertyKind::SignedInt(int_size, Some(flux.read_isized(int_size)?))
                        },
                    )
                } else {
                    (
                        (int_size as usize) as u16,
                        if proptype == 0b0010 {
                            PropertyKind::UnsignedInt(int_size, None)
                        } else {
                            PropertyKind::SignedInt(int_size, None)
                        },
                    )
                }
            }
            0b0100 => {
                todo!() // Redirection and SubRange
            }
            0b0101 => {
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
                    (0, {
                        let size = flux.read_usized(size_size.unwrap())?;
                        let mut fixed_data = [0; 31];
                        flux.read_exact(&mut fixed_data[..fixed_array_size as usize])?;
                        let key_id = if key_size != 0 {
                            Some(flux.read_usized(ByteSize::try_from(key_size as usize).unwrap())?)
                        } else {
                            None
                        };
                        PropertyKind::Array(
                            size_size,
                            fixed_array_size,
                            deported_info,
                            Some((size, fixed_data, key_id)),
                        )
                    })
                } else {
                    (
                        match size_size {
                            None => 0,
                            Some(s) => s as usize as u16,
                        } + fixed_array_size as u16
                            + key_size as u16,
                        PropertyKind::Array(size_size, fixed_array_size, deported_info, None),
                    )
                }
            }
            0b1000 => (1, PropertyKind::VariantId),
            0b1010 | 0b1011 => {
                let default_value = (propdata & 0b1000) != 0;
                let int_size = ByteSize::try_from((propdata & 0x07) as usize + 1).unwrap();
                let key_id_size =
                    ByteSize::try_from((flux.read_u8()? & 0x07) as usize + 1).unwrap();
                let key_store_id = ValueStoreIdx::from(flux.read_u8()?);
                if default_value {
                    let key_value = flux.read_usized(key_id_size)?;
                    (
                        0,
                        if proptype == 0b0010 {
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
                    )
                } else {
                    (
                        (int_size as usize) as u16,
                        if proptype == 0b0010 {
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
                    )
                }
            }
            _ => {
                return Err(format_error!(
                    &format!("Invalid property type ({proptype})"),
                    flux
                ))
            }
        };
        Ok(Self {
            size: propsize as usize,
            kind,
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

    #[test_case(&[0b1000_0000] => RawProperty{size:1, kind:PropertyKind::VariantId })]
    #[test_case(&[0b0000_0000] => RawProperty{size:1, kind:PropertyKind::Padding })]
    #[test_case(&[0b0000_0111] => RawProperty{size:8, kind:PropertyKind::Padding })]
    #[test_case(&[0b0000_1111] => RawProperty{size:16, kind:PropertyKind::Padding })]
    #[test_case(&[0b0001_0100] => RawProperty{size:2, kind:PropertyKind::ContentAddress(ByteSize::U1, None) })]
    #[test_case(&[0b0001_0101] => RawProperty{size:3, kind:PropertyKind::ContentAddress(ByteSize::U2, None) })]
    #[test_case(&[0b0001_0110] => RawProperty{size:4, kind:PropertyKind::ContentAddress(ByteSize::U3, None) })]
    #[test_case(&[0b0001_0000, 0x01] => RawProperty{size:1, kind:PropertyKind::ContentAddress(ByteSize::U1, Some(1.into())) })]
    #[test_case(&[0b0001_0001, 0x01] => RawProperty{size:2, kind:PropertyKind::ContentAddress(ByteSize::U2, Some(1.into())) })]
    #[test_case(&[0b0001_0010, 0x01] => RawProperty{size:3, kind:PropertyKind::ContentAddress(ByteSize::U3, Some(1.into())) })]
    #[test_case(&[0b0010_0000] => RawProperty{size:1, kind:PropertyKind::UnsignedInt(ByteSize::U1, None) })]
    #[test_case(&[0b0010_0010] => RawProperty{size:3, kind:PropertyKind::UnsignedInt(ByteSize::U3, None) })]
    #[test_case(&[0b0010_0111] => RawProperty{size:8, kind:PropertyKind::UnsignedInt(ByteSize::U8, None) })]
    #[test_case(&[0b0011_0000] => RawProperty{size:1, kind:PropertyKind::SignedInt(ByteSize::U1, None) })]
    #[test_case(&[0b0011_0010] => RawProperty{size:3, kind:PropertyKind::SignedInt(ByteSize::U3, None) })]
    #[test_case(&[0b0011_0111] => RawProperty{size:8, kind:PropertyKind::SignedInt(ByteSize::U8, None) })]
    // Plain integer with default value
    #[test_case(&[0b0010_1000, 0xff] => RawProperty{size:0, kind:PropertyKind::UnsignedInt(ByteSize::U1, Some(0xff)) })]
    #[test_case(&[0b0010_1010, 0x01, 0x02, 0x03] => RawProperty{size:0, kind:PropertyKind::UnsignedInt(ByteSize::U3, Some(0x010203)) })]
    #[test_case(&[0b0010_1111, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08] => RawProperty{size:0, kind:PropertyKind::UnsignedInt(ByteSize::U8, Some(0x0102030405060708)) })]
    #[test_case(&[0b0011_1000, 0xff] => RawProperty{size:0, kind:PropertyKind::SignedInt(ByteSize::U1, Some(-1_i64)) })]
    #[test_case(&[0b0011_1010, 0x01, 0x02, 0x03] => RawProperty{size:0, kind:PropertyKind::SignedInt(ByteSize::U3, Some(0x010203_i64)) })]
    #[test_case(&[0b0011_1111, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08] => RawProperty{size:0, kind:PropertyKind::SignedInt(ByteSize::U8, Some(0x0102030405060708_i64)) })]
    // Char[] without deported part :
    #[test_case(&[0b0101_0001, 0b000_00000] => RawProperty{size:1+0+0, kind:PropertyKind::Array(Some(ByteSize::U1), 0, None, None) })]
    #[test_case(&[0b0101_0001, 0b000_00001] => RawProperty{size:1+1+0, kind:PropertyKind::Array(Some(ByteSize::U1), 1, None, None) })]
    #[test_case(&[0b0101_0011, 0b000_00101] => RawProperty{size:3+5+0, kind:PropertyKind::Array(Some(ByteSize::U3), 5, None, None) })]
    #[test_case(&[0b0101_0111, 0b000_11111] => RawProperty{size:7+31+0, kind:PropertyKind::Array(Some(ByteSize::U7), 31, None, None) })]
    // Char[] with deported part :
    #[test_case(&[0b0101_0001, 0b001_00000, 0x0F] => RawProperty{size:1+0+1, kind:PropertyKind::Array(Some(ByteSize::U1), 0, Some((ByteSize::U1, ValueStoreIdx::from(0x0F))), None) })]
    #[test_case(&[0b0101_0001, 0b010_00001, 0x0F] => RawProperty{size:1+1+2, kind:PropertyKind::Array(Some(ByteSize::U1), 1, Some((ByteSize::U2, ValueStoreIdx::from(0x0F))), None) })]
    #[test_case(&[0b0101_0011, 0b100_00101, 0x0F] => RawProperty{size:3+5+4, kind:PropertyKind::Array(Some(ByteSize::U3), 5, Some((ByteSize::U4, ValueStoreIdx::from(0x0F))), None) })]
    #[test_case(&[0b0101_0111, 0b100_11111, 0x0F] => RawProperty{size:7+31+4, kind:PropertyKind::Array(Some(ByteSize::U7), 31, Some((ByteSize::U4, ValueStoreIdx::from(0x0F))), None) })]
    fn test_rawproperty(source: &[u8]) -> RawProperty {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        RawProperty::produce(&mut flux).unwrap()
    }
}
