use crate::bases::*;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
pub struct ContainerPackHeader {
    pub pack_locators_pos: Offset,
    pub pack_count: PackCount,
    pub free_data: PackFreeData,
}

impl ContainerPackHeader {
    pub fn new(pack_locators_pos: Offset, pack_count: PackCount, free_data: PackFreeData) -> Self {
        ContainerPackHeader {
            pack_locators_pos,
            pack_count,
            free_data,
        }
    }
}

impl Parsable for ContainerPackHeader {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let pack_locators_pos = Offset::parse(parser)?;
        let pack_count = parser.read_u16()?.into();
        parser.skip(26)?;
        let free_data = PackFreeData::parse(parser)?;
        Ok(ContainerPackHeader {
            pack_locators_pos,
            pack_count,
            free_data,
        })
    }
}

impl BlockParsable for ContainerPackHeader {}

impl SizedParsable for ContainerPackHeader {
    const SIZE: usize = Offset::SIZE
         + 2 // packCount
         + 26 //padding
         + PackFreeData::SIZE;
}

impl Serializable for ContainerPackHeader {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += self.pack_locators_pos.serialize(ser)?;
        written += self.pack_count.serialize(ser)?;
        written += ser.write_data(&[0_u8; 26])?;
        written += self.free_data.serialize(ser)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_containerpackheader() {
        let mut content = vec![
            0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // pack_locators_pos
            0x05, 0x00, // pack_count
        ];
        content.extend_from_slice(&[0; 26]); // padding
        content.extend_from_slice(&[0xFF; 24]); // free_data
        content.extend_from_slice(&[0x00; 4]); // Dummy CRC32
        let reader = Reader::from(content);
        let container_header = reader
            .parse_block_at::<ContainerPackHeader>(Offset::zero())
            .unwrap();
        assert_eq!(
            container_header,
            ContainerPackHeader {
                pack_locators_pos: Offset::from(0xffff_u64),
                pack_count: 0x05_u16.into(),
                free_data: PackFreeData::from([0xFF; 24])
            }
        );
    }
}
