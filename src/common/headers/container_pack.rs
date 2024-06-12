use crate::bases::*;
use crate::common::{FullPackKind, PackKind};
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
pub struct ContainerPackHeader {
    pub version: u8,
    pub pack_count: PackCount,
    pub file_size: Size,
}

impl ContainerPackHeader {
    pub fn new(pack_count: PackCount, file_size: Size) -> Self {
        ContainerPackHeader {
            pack_count,
            file_size,
            version: 0,
        }
    }
}

impl Parsable for ContainerPackHeader {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let magic = FullPackKind::parse(parser)?;
        if magic != PackKind::Container {
            return Err(format_error!("Pack Magic is not ContainerPack"));
        }
        let version = parser.read_u8()?;
        let pack_count = parser.read_u16()?.into();
        parser.skip(1)?;
        let file_size = Size::parse(parser)?;
        Ok(ContainerPackHeader {
            version,
            pack_count,
            file_size,
        })
    }
}

impl BlockParsable for ContainerPackHeader {}

impl SizedParsable for ContainerPackHeader {
    const SIZE: usize = FullPackKind::SIZE
         + 1 // version
         + 2 // packCount
         + 1 //padding
         + Size::SIZE;
}

impl Serializable for ContainerPackHeader {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += FullPackKind(PackKind::Container).serialize(ser)?;
        written += ser.write_u8(self.version)?;
        written += ser.write_u16(self.pack_count.into_u16())?;
        written += ser.write_data(&[0_u8; 1])?;
        written += self.file_size.serialize(ser)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_containerpackheader() {
        let content = vec![
            0x6a, 0x62, 0x6b, 0x43, // magic
            0x01, // version
            0x05, 0x00, // pack_count
            0x00, // padding
            0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // file_size
        ];
        let reader = Reader::from(content);
        let container_header = reader
            .parse_block_at::<ContainerPackHeader>(Offset::zero())
            .unwrap();
        assert_eq!(
            container_header,
            ContainerPackHeader {
                version: 0x01_u8,
                pack_count: 0x05_u16.into(),
                file_size: Size::from(0xffff_u64),
            }
        );
    }
}
