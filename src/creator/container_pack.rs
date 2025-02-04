use camino::{Utf8Path, Utf8PathBuf};

use crate::bases::*;
use crate::common::{CheckInfo, ContainerPackHeader, PackHeader, PackHeaderInfo, PackLocator};
use crate::creator::Result;
use std::io::{self, Read, Seek, SeekFrom, Write};

use super::private::Sealed;
use super::{MaybeFileReader, NamedFile, PackRecipient};

pub struct ContainerPackCreator<F: PackRecipient> {
    packs: Vec<PackLocator>,
    file: Box<F>,
    free_data: PackFreeData,
}

#[derive(Debug)]
pub struct InContainerFile<F: PackRecipient> {
    file: Skip<Box<F>>,
    packs: Vec<PackLocator>,
    container_free_data: PackFreeData,
}

impl ContainerPackCreator<NamedFile> {
    pub fn new(path: impl AsRef<Utf8Path>, free_data: PackFreeData) -> IoResult<Self> {
        Self::from_file(NamedFile::new(path)?, free_data)
    }
}
impl<F: PackRecipient> ContainerPackCreator<F> {
    pub fn from_file(mut file: Box<F>, free_data: PackFreeData) -> IoResult<Self> {
        file.seek(SeekFrom::Start(
            (PackHeader::BLOCK_SIZE + ContainerPackHeader::BLOCK_SIZE) as u64,
        ))?;
        Ok(ContainerPackCreator {
            packs: vec![],
            file,
            free_data,
        })
    }

    pub fn into_file(self) -> IoResult<Box<InContainerFile<F>>> {
        Ok(Box::new(self::InContainerFile {
            file: Skip::new(self.file)?,
            packs: self.packs,
            container_free_data: self.free_data,
        }))
    }

    pub fn add_pack<I: Read>(&mut self, uuid: uuid::Uuid, reader: &mut I) -> IoResult<()> {
        let pack_offset = self.file.tell();
        std::io::copy(reader, &mut self.file)?;
        let pack_size = self.file.tell() - pack_offset;
        let pack_locator = PackLocator::new(uuid, pack_size, pack_offset);
        self.packs.push(pack_locator);
        Ok(())
    }

    pub fn finalize(mut self) -> IoResult<Box<F>> {
        let pack_locators_pos = self.file.tell();

        for pack_locator in &self.packs {
            self.file.ser_write(pack_locator)?;
        }

        let check_info_pos = self.file.tell();

        // Write pack header
        let pack_size = Size::from(check_info_pos + PackHeader::BLOCK_SIZE);
        let pack_header = PackHeader::new(
            crate::common::PackKind::Container,
            PackHeaderInfo::new(VendorId::from([0, 0, 0, 0]), pack_size, check_info_pos),
        );
        self.file.rewind()?;
        self.file.ser_write(&pack_header)?;

        // Write container pack header
        let header = ContainerPackHeader::new(
            pack_locators_pos,
            PackCount::from(self.packs.len() as u16),
            self.free_data,
        );
        self.file.ser_write(&header)?;

        self.file.seek(SeekFrom::End(0))?;
        let check_info = CheckInfo::new_none();
        self.file.ser_write(&check_info)?;

        self.file.rewind()?;
        let mut tail_buffer = [0u8; PackHeader::BLOCK_SIZE];
        self.file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&tail_buffer)?;

        Ok(self.file)
    }
}

impl<F: PackRecipient> InContainerFile<F> {
    pub fn close(mut self, uuid: uuid::Uuid) -> IoResult<ContainerPackCreator<F>> {
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
            free_data: self.container_free_data,
        })
    }
}

impl<F: PackRecipient> Seek for InContainerFile<F> {
    fn seek(&mut self, pos: io::SeekFrom) -> IoResult<u64> {
        self.file.seek(pos)
    }
}

impl<F: PackRecipient> Write for InContainerFile<F> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.file.flush()
    }
}

impl<F: PackRecipient> Read for InContainerFile<F> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.file.read(buf)
    }
}

impl<F: PackRecipient + std::fmt::Debug + Sync + Send> OutStream for InContainerFile<F> {
    fn copy(
        &mut self,
        reader: Box<dyn crate::creator::InputReader>,
    ) -> IoResult<(u64, MaybeFileReader)> {
        self.file.copy(reader)
    }
}

impl<F: PackRecipient + Sync + Send> Sealed for InContainerFile<F> {}

impl<F: PackRecipient + Sync + Send> PackRecipient for InContainerFile<F> {
    fn close_file(self: Box<Self>) -> Result<Utf8PathBuf> {
        Ok(Utf8PathBuf::new())
    }
}
