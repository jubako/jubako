use crate::bases::*;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DirectoryPackHeader {
    pub index_ptr_pos: Offset,
    pub entry_store_ptr_pos: Offset,
    pub value_store_ptr_pos: Offset,
    pub index_count: IndexCount,
    pub entry_store_count: EntryStoreCount,
    pub value_store_count: ValueStoreCount,
    pub free_data: PackFreeData,
}

impl DirectoryPackHeader {
    pub fn new(
        free_data: PackFreeData,
        indexes: (IndexCount, Offset),
        value_stores: (ValueStoreCount, Offset),
        entry_stores: (EntryStoreCount, Offset),
    ) -> Self {
        DirectoryPackHeader {
            index_ptr_pos: indexes.1,
            index_count: indexes.0,
            value_store_ptr_pos: value_stores.1,
            value_store_count: value_stores.0,
            entry_store_ptr_pos: entry_stores.1,
            entry_store_count: entry_stores.0,
            free_data,
        }
    }
}

impl Parsable for DirectoryPackHeader {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let index_ptr_pos = Offset::parse(parser)?;
        let entry_store_ptr_pos = Offset::parse(parser)?;
        let value_store_ptr_pos = Offset::parse(parser)?;
        let index_count = Count::<u32>::parse(parser)?.into();
        let entry_store_count = Count::<u32>::parse(parser)?.into();
        let value_store_count = Count::<u8>::parse(parser)?.into();
        parser.skip(3)?;
        let free_data = PackFreeData::parse(parser)?;
        Ok(DirectoryPackHeader {
            entry_store_ptr_pos,
            value_store_ptr_pos,
            index_ptr_pos,
            entry_store_count,
            index_count,
            value_store_count,
            free_data,
        })
    }
}

impl BlockParsable for DirectoryPackHeader {}

impl SizedParsable for DirectoryPackHeader {
    const SIZE: usize = Offset::SIZE
        + Offset::SIZE
        + Offset::SIZE
        + Count::<u32>::SIZE
        + Count::<u32>::SIZE
        + Count::<u8>::SIZE
        + 3 // padding
        + PackFreeData::SIZE;
}

impl Serializable for DirectoryPackHeader {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += self.index_ptr_pos.serialize(ser)?;
        written += self.entry_store_ptr_pos.serialize(ser)?;
        written += self.value_store_ptr_pos.serialize(ser)?;
        written += self.index_count.serialize(ser)?;
        written += self.entry_store_count.serialize(ser)?;
        written += self.value_store_count.serialize(ser)?;
        written += ser.write_data(&[0; 3])?;
        written += self.free_data.serialize(ser)?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directorypackheader() {
        let mut content = vec![
            0xdd, 0xee, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // index_ptr_pos
            0x00, 0xee, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // entry_store_ptr_pos
            0xaa, 0xee, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // value_store_ptr_pos
            0x50, 0x00, 0x00, 0x00, // index count
            0x60, 0x00, 0x00, 0x00, // entry_store count
            0x05, //value_store count
        ];
        content.extend_from_slice(&[0x00; 3]);
        content.extend_from_slice(&[0xff; 24]);
        content.extend_from_slice(&[0xBD, 0x7A, 0xC5, 0x13]); // CRC32
        let reader = Reader::from(content);
        let directory_pack_header = reader
            .parse_block_at::<DirectoryPackHeader>(Offset::zero())
            .unwrap();
        assert_eq!(
            directory_pack_header,
            DirectoryPackHeader {
                index_ptr_pos: Offset::from(0xeedd_u64),
                entry_store_ptr_pos: Offset::from(0xee00_u64),
                value_store_ptr_pos: Offset::from(0xeeaa_u64),
                index_count: IndexCount::from(0x50_u32),
                entry_store_count: EntryStoreCount::from(0x60_u32),
                value_store_count: ValueStoreCount::from(0x05_u8),
                free_data: [0xff; 24].into(),
            }
        );
    }
}
