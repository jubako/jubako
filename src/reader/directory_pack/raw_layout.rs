use crate::bases::*;

/// The kind of property definition as specified in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawPropertyKind {
    Padding,
    ContentAddress,
    UnsignedInt,
    SignedInt,
    Array,
    VLArray(bool /*flookup*/, u8 /*idx*/),
    VariantId,
}

/// The property definition as defined in the store.
/// The RawProperty is somehow independent of the other properties :
/// It may be sementically dependent but this structure doesn't represent it.
/// The resolition of the depedencies is made later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawProperty {
    pub size: usize,
    pub kind: RawPropertyKind,
}

impl RawProperty {
    pub fn new(kind: RawPropertyKind, size: usize) -> Self {
        Self { size, kind }
    }

    pub fn is_variant_id(&self) -> bool {
        self.kind == RawPropertyKind::VariantId
    }
}

impl Producable for RawProperty {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let propinfo = flux.read_u8()?;
        let proptype = propinfo >> 4;
        let propdata = propinfo & 0x0F;
        let (propsize, kind) = match proptype {
            0b0000 => (propdata as u16 + 1, RawPropertyKind::Padding),
            0b0001 => (4, RawPropertyKind::ContentAddress),
            0b0010 => (
                (propdata & 0x07) as u16 + 1,
                if (propdata & 0x08) != 0 {
                    RawPropertyKind::SignedInt
                } else {
                    RawPropertyKind::UnsignedInt
                },
            ),
            0b0100 => {
                (
                    if propdata & 0x08 == 0 {
                        (propdata + 1) as u16
                    } else {
                        // We need a complement byte
                        let complement = flux.read_u8()?;
                        (((propdata & 0x03) as u16) << 8) + complement as u16 + 9
                    },
                    RawPropertyKind::Array,
                )
            }
            0b0110 | 0b0111 => {
                let flookup: bool = proptype & 0b1 != 0;
                let size = propdata as u16 + 1;
                let keystoreidx = flux.read_u8()?;
                (size, RawPropertyKind::VLArray(flookup, keystoreidx))
            }
            0b1000 => (1, RawPropertyKind::VariantId),
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
        let raw_property_count: PropertyCount = Count::<u8>::produce(flux)?.into();
        let mut raw_properties = Vec::with_capacity(raw_property_count.into_usize());
        for _ in raw_property_count {
            let raw_property = RawProperty::produce(flux)?;
            raw_properties.push(raw_property);
        }
        Ok(Self(raw_properties))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(&[0b1000_0000] => RawProperty{size:1, kind:RawPropertyKind::VariantId })]
    #[test_case(&[0b0000_0000] => RawProperty{size:1, kind:RawPropertyKind::Padding })]
    #[test_case(&[0b0000_0111] => RawProperty{size:8, kind:RawPropertyKind::Padding })]
    #[test_case(&[0b0001_0000] => RawProperty{size:4, kind:RawPropertyKind::ContentAddress })]
    #[test_case(&[0b0010_0000] => RawProperty{size:1, kind:RawPropertyKind::UnsignedInt })]
    #[test_case(&[0b0010_0010] => RawProperty{size:3, kind:RawPropertyKind::UnsignedInt })]
    #[test_case(&[0b0010_0111] => RawProperty{size:8, kind:RawPropertyKind::UnsignedInt })]
    #[test_case(&[0b0010_1000] => RawProperty{size:1, kind:RawPropertyKind::SignedInt })]
    #[test_case(&[0b0010_1010] => RawProperty{size:3, kind:RawPropertyKind::SignedInt })]
    #[test_case(&[0b0010_1111] => RawProperty{size:8, kind:RawPropertyKind::SignedInt })]
    #[test_case(&[0b0100_0000] => RawProperty{size:1, kind:RawPropertyKind::Array })]
    #[test_case(&[0b0100_0111] => RawProperty{size:8, kind:RawPropertyKind::Array })]
    #[test_case(&[0b0100_1000, 0x00]=> RawProperty{size:9, kind:RawPropertyKind::Array })]
    #[test_case(&[0b0100_1000, 0xFF]=> RawProperty{size:264, kind:RawPropertyKind::Array })]
    #[test_case(&[0b0100_1011, 0xFF]=> RawProperty{size:1032, kind:RawPropertyKind::Array })]
    #[test_case(&[0b0110_0000, 0x0F]=> RawProperty{size:1, kind:RawPropertyKind::VLArray(false, 0x0F) })]
    #[test_case(&[0b0110_0111, 0x0F]=> RawProperty{size:8, kind:RawPropertyKind::VLArray(false, 0x0F) })]
    #[test_case(&[0b0111_0000, 0x0F]=> RawProperty{size:1, kind:RawPropertyKind::VLArray(true, 0x0F) })]
    #[test_case(&[0b0111_0111, 0x0F]=> RawProperty{size:8, kind:RawPropertyKind::VLArray(true, 0x0F) })]
    fn test_rawproperty(source: &[u8]) -> RawProperty {
        let mut content = Vec::new();
        content.extend_from_slice(source);
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        RawProperty::produce(&mut flux).unwrap()
    }
}
