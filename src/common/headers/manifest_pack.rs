use crate::bases::*;
use crate::common::{PackHeader, PackHeaderInfo, PackInfo, PackKind};

#[derive(Debug, PartialEq, Eq)]
pub struct ManifestPackHeader {
    pub pack_header: PackHeader,
    pub pack_count: PackCount,
    pub value_store_posinfo: SizedOffset,
    pub free_data: ManifestPackFreeData,
}

impl ManifestPackHeader {
    pub fn new(
        pack_info: PackHeaderInfo,
        free_data: ManifestPackFreeData,
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

    pub fn packs_offset(&self) -> Offset {
        Offset::from(
            self.pack_header.check_info_pos.into_u64()
                - self.pack_count.into_u64() * PackInfo::SIZE as u64,
        )
    }
}

impl SizedProducable for ManifestPackHeader {
    const SIZE: usize =
        PackHeader::SIZE + Count::<u8>::SIZE + SizedOffset::SIZE + ManifestPackFreeData::SIZE;
}

impl Producable for ManifestPackHeader {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let pack_header = PackHeader::produce(flux)?;
        if pack_header.magic != PackKind::Manifest {
            return Err(format_error!("Pack Magic is not ManifestPack"));
        }
        let pack_count = Count::<u8>::produce(flux)?.into();
        let value_store_posinfo = SizedOffset::produce(flux)?;
        let free_data = ManifestPackFreeData::produce(flux)?;
        Ok(Self {
            pack_header,
            pack_count,
            value_store_posinfo,
            free_data,
        })
    }
}

impl Writable for ManifestPackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += self.pack_header.write(stream)?;
        written += self.pack_count.write(stream)?;
        written += self.value_store_posinfo.write(stream)?;
        written += self.free_data.write(stream)?;
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
            0x01, 0x00, 0x00, 0x00, // app_vendor_id
            0x01, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, // file_size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xee, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x02, // pack_count
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Valuestoreoffset
        ];
        content.extend_from_slice(&[0xff; 55]);
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        assert_eq!(
            ManifestPackHeader::produce(&mut flux).unwrap(),
            ManifestPackHeader {
                pack_header: PackHeader {
                    magic: PackKind::Manifest,
                    app_vendor_id: 0x01000000_u32,
                    major_version: 0x01_u8,
                    minor_version: 0x02_u8,
                    uuid: Uuid::from_bytes([
                        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
                        0x0c, 0x0d, 0x0e, 0x0f
                    ]),
                    file_size: Size::from(0xffff_u64),
                    check_info_pos: Offset::from(0xffee_u64),
                },
                pack_count: PackCount::from(2),
                value_store_posinfo: SizedOffset::new(Size::zero(), Offset::zero()),
                free_data: [0xff; 55].into(),
            }
        );
    }
}
