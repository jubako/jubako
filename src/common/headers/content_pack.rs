use crate::bases::*;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
pub struct ContentPackHeader {
    pub content_ptr_pos: Offset,
    pub cluster_ptr_pos: Offset,
    pub content_count: ContentCount,
    pub cluster_count: ClusterCount,
    pub free_data: PackFreeData,
}

impl ContentPackHeader {
    pub fn new(
        free_data: PackFreeData,
        cluster_ptr_pos: Offset,
        cluster_count: ClusterCount,
        content_ptr_pos: Offset,
        content_count: ContentCount,
    ) -> Self {
        Self {
            content_ptr_pos,
            cluster_ptr_pos,
            content_count,
            cluster_count,
            free_data,
        }
    }
}

impl Parsable for ContentPackHeader {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let content_ptr_pos = Offset::parse(parser)?;
        let cluster_ptr_pos = Offset::parse(parser)?;
        let content_count = Count::<u32>::parse(parser)?.into();
        let cluster_count = Count::<u32>::parse(parser)?.into();
        parser.skip(12)?;
        let free_data = PackFreeData::parse(parser)?;
        parser.skip(4)?;
        Ok(ContentPackHeader {
            content_ptr_pos,
            cluster_ptr_pos,
            content_count,
            cluster_count,
            free_data,
        })
    }
}

impl SizedParsable for ContentPackHeader {
    const SIZE: usize = Offset::SIZE
        + Offset::SIZE
        + 4 // ContentCount::SIZE
        + 4 // ClusterCount::SIZE
        + 12 // Padding
        + PackFreeData::SIZE
  + 4; // reserved
}

impl BlockParsable for ContentPackHeader {}

impl Serializable for ContentPackHeader {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += self.content_ptr_pos.serialize(ser)?;
        written += self.cluster_ptr_pos.serialize(ser)?;
        written += self.content_count.serialize(ser)?;
        written += self.cluster_count.serialize(ser)?;
        written += ser.write_data(&[0; 12])?;
        written += self.free_data.serialize(ser)?;
        written += ser.write_data(&[0; 4])?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contentpackheader() {
        let mut content = vec![
            0x00, 0xee, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // entry_ptr_pos
            0xdd, 0xee, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // cluster_ptr_pos
            0x50, 0x00, 0x00, 0x00, // entry ccount
            0x60, 0x00, 0x00, 0x00, // cluster ccount
        ];
        content.extend_from_slice(&[0x00; 12]); // padding
        content.extend_from_slice(&[0xff; 24]); // free_data
        content.extend_from_slice(&[0x00; 4]);
        let reader = Reader::from(content);
        let content_pack_header = reader
            .parse_block_at::<ContentPackHeader>(Offset::zero())
            .unwrap();
        assert_eq!(
            content_pack_header,
            ContentPackHeader {
                content_ptr_pos: Offset::from(0xee00_u64),
                cluster_ptr_pos: Offset::from(0xeedd_u64),
                content_count: ContentCount::from(0x50_u32),
                cluster_count: ClusterCount::from(0x60_u32),
                free_data: [0xff; 24].into(),
            }
        );
    }
}
