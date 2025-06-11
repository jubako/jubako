use crate::bases::*;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ManifestPackHeader {
    pub pack_count: PackCount,
    pub value_store_posinfo: SizedOffset,
    pub free_data: PackFreeData,
}

impl ManifestPackHeader {
    pub fn new(
        free_data: PackFreeData,
        pack_count: PackCount,
        value_store_posinfo: SizedOffset,
    ) -> Self {
        ManifestPackHeader {
            pack_count,
            value_store_posinfo,
            free_data,
        }
    }
}

impl SizedParsable for ManifestPackHeader {
    const SIZE: usize = Count::<u16>::SIZE // pack_count
      + SizedOffset::SIZE // value_store_posinfo
      + 26 // padding
      + PackFreeData::SIZE;
}

impl Parsable for ManifestPackHeader {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let pack_count = Count::<u16>::parse(parser)?.into();
        let value_store_posinfo = SizedOffset::parse(parser)?;
        parser.skip(26)?;
        let free_data = PackFreeData::parse(parser)?;

        Ok(Self {
            pack_count,
            value_store_posinfo,
            free_data,
        })
    }
}

impl BlockParsable for ManifestPackHeader {}

impl Serializable for ManifestPackHeader {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += self.pack_count.serialize(ser)?;
        written += self.value_store_posinfo.serialize(ser)?;
        written += ser.write_data(&[0; 26])?;
        written += self.free_data.serialize(ser)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustest::test]
    fn test_mainpackheader() {
        let mut content = vec![
            0x02, 0x00, // pack_count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Valuestoreoffset
        ];
        content.extend_from_slice(&[0x00; 26]);
        content.extend_from_slice(&[0xff; 24]);
        content.extend_from_slice(&[0x91, 0xAF, 0xA1, 0xBC]); // CRC32
        let reader = Reader::from(content);
        let manifest_pack_header = reader
            .parse_block_at::<ManifestPackHeader>(Offset::zero())
            .unwrap();
        assert_eq!(
            manifest_pack_header,
            ManifestPackHeader {
                pack_count: PackCount::from(2),
                value_store_posinfo: SizedOffset::new(0.into(), Offset::zero()),
                free_data: [0xff; 24].into(),
            }
        );
    }
}
