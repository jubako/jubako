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

impl Producable for ContainerPackHeader {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let magic = FullPackKind::produce(flux)?;
        if magic != PackKind::Container {
            return Err(format_error!("Pack Magic is not ContainerPack"));
        }
        let version = flux.read_u8()?;
        let pack_count = flux.read_u8()?.into();
        flux.skip(Size::new(2))?;
        let file_size = Size::produce(flux)?;
        Ok(ContainerPackHeader {
            version,
            pack_count,
            file_size,
        })
    }
}

impl SizedProducable for ContainerPackHeader {
    const SIZE: usize = FullPackKind::SIZE
         + 1 // version
         + 1 // packCount
         + 2 //padding
         + Size::SIZE;
}

impl Writable for ContainerPackHeader {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += FullPackKind(PackKind::Container).write(stream)?;
        written += stream.write_u8(self.version)?;
        written += stream.write_u8(self.pack_count.into_u8())?;
        written += stream.write_data(&[0_u8; 2])?;
        written += self.file_size.write(stream)?;
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
            0x05, // pack_count
            0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, // file_size
        ];
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        assert_eq!(
            ContainerPackHeader::produce(&mut flux).unwrap(),
            ContainerPackHeader {
                version: 0x01_u8,
                pack_count: 0x05_u8.into(),
                file_size: Size::from(0xffff_u64),
            }
        );
    }
}
