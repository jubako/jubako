use crate::bases::*;
use crate::common::{ContainerPackHeader, PackLocator};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub struct ContainerPackCreator {
    packs: Vec<PackLocator>,
    file: File,
}

#[derive(Debug)]
pub struct InContainerFile {
    file: Skip<File>,
    packs: Vec<PackLocator>,
}

const HEADER_SIZE: u64 = ContainerPackHeader::SIZE as u64;

impl ContainerPackCreator {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        Self::from_file(file)
    }

    pub fn from_file(mut file: File) -> Result<Self> {
        file.seek(SeekFrom::Start(HEADER_SIZE))?;
        Ok(ContainerPackCreator {
            packs: vec![],
            file,
        })
    }

    pub fn into_file(self) -> Result<InContainerFile> {
        Ok(InContainerFile {
            file: Skip::new(self.file)?,
            packs: self.packs,
        })
    }

    pub fn add_pack<I: Read>(&mut self, uuid: uuid::Uuid, reader: &mut I) -> Result<()> {
        let pack_offset = self.file.tell();
        std::io::copy(reader, &mut self.file)?;
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
        let header = ContainerPackHeader::new(PackCount::from(self.packs.len() as u16), pack_size);
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

impl InContainerFile {
    pub fn close(mut self, uuid: uuid::Uuid) -> Result<ContainerPackCreator> {
        let pack_size = self.file.seek(SeekFrom::End(0))?;
        self.file.seek(SeekFrom::Start(0))?;
        let mut file = self.file.into_inner();
        let pack_offset = file.stream_position()?;
        file.seek(SeekFrom::End(0))?;
        let pack_locator = PackLocator::new(uuid, pack_size.into(), pack_offset.into());
        self.packs.push(pack_locator);
        Ok(ContainerPackCreator {
            file,
            packs: self.packs,
        })
    }
}

impl Seek for InContainerFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Write for InContainerFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Read for InContainerFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl OutStream for InContainerFile {
    fn copy(&mut self, reader: Box<dyn crate::creator::InputReader>) -> IoResult<u64> {
        self.file.copy(reader)
    }
}
