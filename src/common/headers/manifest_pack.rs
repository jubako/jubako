use crate::bases::*;
use crate::common::{PackHeader, PackHeaderInfo, PackInfo, PackKind};

pub struct PackOffsetsIter {
    offset: Offset,
    left: u16,
}

impl PackOffsetsIter {
    fn new(check_info_pos: Offset, pack_count: PackCount) -> Self {
        let offset = Offset::from(
            check_info_pos.into_u64() - pack_count.into_u64() * (PackInfo::SIZE as u64/* + 4*/),
        );
        Self {
            offset,
            left: pack_count.into_u16(),
        }
    }
}

impl Iterator for PackOffsetsIter {
    type Item = Offset;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left != 0 {
            let offset = self.offset;
            self.offset += PackInfo::SIZE;
            self.left -= 1;
            Some(offset)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ManifestPackHeader {
    pub pack_header: PackHeader,
    pub pack_count: PackCount,
    pub value_store_posinfo: SizedOffset,
    pub free_data: PackFreeData,
}

impl ManifestPackHeader {
    pub fn new(
        pack_info: PackHeaderInfo,
        free_data: PackFreeData,
        pack_count: PackCount,
        value_store_posinfo: SizedOffset,
    ) -> Self {
        ManifestPackHeader {
            pack_header: PackHeader::new(PackKind::Manifest, pack_info),
            pack_count,
            value_store_posinfo,
            free_data,
        }
    }

    pub fn packs_offset(&self) -> PackOffsetsIter {
        PackOffsetsIter::new(self.pack_header.check_info_pos, self.pack_count)
    }
}

impl SizedParsable for ManifestPackHeader {
    const SIZE: usize = PackHeader::SIZE + Count::<u16>::SIZE // pack_count
        + SizedOffset::SIZE // value store_posinfo
        + 26 // padding
        + PackFreeData::SIZE
        + 4; // padding;
}

impl Parsable for ManifestPackHeader {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let pack_header = PackHeader::parse(parser)?;
        if pack_header.magic != PackKind::Manifest {
            return Err(format_error!("Pack Magic is not ManifestPack"));
        }
        let pack_count = Count::<u16>::parse(parser)?.into();
        let value_store_posinfo = SizedOffset::parse(parser)?;
        parser.skip(26)?;
        let free_data = PackFreeData::parse(parser)?;
        parser.skip(4)?;

        Ok(Self {
            pack_header,
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
        written += self.pack_header.serialize(ser)?;
        written += self.pack_count.serialize(ser)?;
        written += self.value_store_posinfo.serialize(ser)?;
        written += ser.write_data(&[0; 26])?;
        written += self.free_data.serialize(ser)?;
        written += ser.write_data(&[0; 4])?;
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_mainpackheader() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x6d, // magic
            0x00, 0x00, 0x00, 0x01, // app_vendor_id
            0x00, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, // flags
            0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // file_size
            0xee, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x02, 0x00, // pack_count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Valuestoreoffset
        ];
        content.extend_from_slice(&[0x00; 26]);
        content.extend_from_slice(&[0xff; 24]);
        content.extend_from_slice(&[0x00; 4]);
        let reader = Reader::from(content);
        let manifest_pack_header = reader
            .parse_block_at::<ManifestPackHeader>(Offset::zero())
            .unwrap();
        assert_eq!(
            manifest_pack_header,
            ManifestPackHeader {
                pack_header: PackHeader {
                    magic: PackKind::Manifest,
                    app_vendor_id: VendorId::from([00, 00, 00, 01]),
                    major_version: 0x00_u8,
                    minor_version: 0x02_u8,
                    uuid: Uuid::from_bytes([
                        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
                        0x0c, 0x0d, 0x0e, 0x0f
                    ]),
                    flags: 0,
                    file_size: Size::from(0xffff_u64),
                    check_info_pos: Offset::from(0xffee_u64),
                },
                pack_count: PackCount::from(2),
                value_store_posinfo: SizedOffset::new(Size::zero(), Offset::zero()),
                free_data: [0xff; 24].into(),
            }
        );
    }
}
