use crate::bases::*;
use crate::common::{ContainerPackHeader, PackLocator};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use typenum::Unsigned;

pub struct ContainerPackCreator {
    packs: Vec<PackLocator>,
    file: File,
}

const HEADER_SIZE: u64 = <ContainerPackHeader as SizedProducable>::Size::U64;

impl ContainerPackCreator {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.seek(SeekFrom::Start(HEADER_SIZE))?;
        Ok(ContainerPackCreator {
            packs: vec![],
            file,
        })
    }

    pub fn add_pack(&mut self, uuid: uuid::Uuid, reader: Reader) -> Result<()> {
        let pack_offset = self.file.tell();
        std::io::copy(&mut reader.create_flux_all(), &mut self.file)?;
        let pack_size = self.file.tell() - pack_offset;
        let pack_locator = PackLocator::new(uuid, pack_size, pack_offset);
        self.packs.push(pack_locator);
        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        for pack_locator in &self.packs {
            pack_locator.write(&mut self.file)?;
        }

        let pack_size: Size = (self.file.tell().into_u64() + HEADER_SIZE).into();

        self.file.rewind()?;
        let header = ContainerPackHeader::new(PackCount::from(self.packs.len() as u8), pack_size);
        header.write(&mut self.file)?;

        self.file.rewind()?;
        let mut tail_buffer = [0u8; HEADER_SIZE as usize];
        self.file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&tail_buffer)?;

        Ok(())
    }
}
