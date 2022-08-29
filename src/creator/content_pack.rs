use super::{CheckInfo, PackInfo};
use crate::bases::*;
use crate::common::{
    ClusterHeader, CompressionType, ContentPackHeader, EntryInfo, PackHeaderInfo, PackPos,
};
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use typenum::U40;

struct ClusterCreator {
    index: usize,
    compression: CompressionType,
    data: Vec<u8>,
    offsets: Vec<usize>,
}

const CLUSTER_SIZE: Size = Size(1024 * 1024 * 4);
const MAX_BLOBS_PER_CLUSTER: usize = 0xFFF;

impl ClusterCreator {
    fn new(index: usize, compression: CompressionType) -> Self {
        ClusterCreator {
            index,
            compression,
            data: Vec::with_capacity(CLUSTER_SIZE.0 as usize),
            offsets: vec![],
        }
    }

    pub fn write_data(&self, stream: &mut dyn OutStream) -> Result<Size> {
        let offset = stream.tell();
        let stream = match &self.compression {
            CompressionType::None => {
                stream.write_data(&self.data)?;
                stream
            }
            CompressionType::Lz4 => {
                let mut encoder = lz4::EncoderBuilder::new().level(16).build(stream)?;
                let mut incursor = Cursor::new(&self.data);
                std::io::copy(&mut incursor, &mut encoder)?;
                let (stream, err) = encoder.finish();
                err?;
                stream
            }
            CompressionType::Lzma => {
                let mut encoder = lzma::LzmaWriter::new_compressor(stream, 9)?;
                let mut incursor = Cursor::new(&self.data);
                std::io::copy(&mut incursor, &mut encoder)?;
                encoder.finish()?
            }
            CompressionType::Zstd => {
                let mut encoder = zstd::Encoder::new(stream, 19)?;
                encoder.multithread(8)?;
                encoder.include_contentsize(false)?;
                //encoder.long_distance_matching(true);
                let mut incursor = Cursor::new(&self.data);
                std::io::copy(&mut incursor, &mut encoder)?;
                encoder.finish()?
            }
        };
        Ok(stream.tell() - offset)
    }

    pub fn write_tail(&self, stream: &mut dyn OutStream, data_size: Size) -> IoResult<()> {
        let offset_size = needed_bytes(self.data.len());
        assert!(offset_size <= 8);
        assert!(offset_size > 0);
        let cluster_header = ClusterHeader::new(
            self.compression,
            offset_size as u8,
            Count(self.offsets.len() as u16),
        );
        cluster_header.write(stream)?;
        stream.write_sized(data_size.0, offset_size)?; // raw data size
        stream.write_sized(self.data.len() as u64, offset_size)?; // datasize
        for offset in &self.offsets[..self.offsets.len() - 1] {
            stream.write_sized(*offset as u64, offset_size)?;
        }
        Ok(())
    }

    pub fn tail_size(&self) -> Size {
        let mut size = 4;
        let size_byte = needed_bytes(self.data.len());
        size += (1 + self.offsets.len()) * size_byte;
        size.into()
    }

    fn data_size(&self) -> Size {
        self.data.len().into()
    }

    pub fn is_full(&self, size: Size) -> bool {
        if self.offsets.len() == MAX_BLOBS_PER_CLUSTER {
            return true;
        }
        !self.offsets.is_empty() && self.data_size() + size > CLUSTER_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn add_content(&mut self, content: &mut dyn Stream) -> IoResult<EntryInfo> {
        assert!(self.offsets.len() < MAX_BLOBS_PER_CLUSTER);
        let idx = self.offsets.len() as u16;
        content.read_to_end(&mut self.data)?;
        self.offsets.push(self.data.len());
        Ok(EntryInfo::new((self.index as u32).into(), Idx(idx)))
    }
}

pub struct ContentPackCreator {
    app_vendor_id: u32,
    pack_id: Id<u8>,
    free_data: FreeData<U40>,
    blob_addresses: Vec<EntryInfo>,
    open_cluster: Option<ClusterCreator>,
    cluster_addresses: Vec<SizedOffset>,
    path: PathBuf,
    file: Option<File>,
    compression: CompressionType,
}

impl ContentPackCreator {
    pub fn new<P: AsRef<Path>>(
        path: P,
        pack_id: Id<u8>,
        app_vendor_id: u32,
        free_data: FreeData<U40>,
        compression: CompressionType,
    ) -> Self {
        ContentPackCreator {
            app_vendor_id,
            pack_id,
            free_data,
            blob_addresses: vec![],
            open_cluster: None,
            cluster_addresses: vec![],
            path: path.as_ref().into(),
            file: None,
            compression,
        }
    }

    fn open_cluster(&mut self) {
        assert!(self.open_cluster.is_none());
        let cluster_id = self.cluster_addresses.len();
        self.open_cluster = Some(ClusterCreator::new(cluster_id, self.compression));
    }

    fn write_cluster(&mut self) -> Result<()> {
        let cluster = self.open_cluster.as_ref().unwrap();
        let data_size = cluster.write_data(self.file.as_mut().unwrap())?;
        let cluster_offset = self.file.as_mut().unwrap().tell();
        cluster.write_tail(self.file.as_mut().unwrap(), data_size)?;
        if self.cluster_addresses.len() <= cluster.index {
            self.cluster_addresses.resize(
                cluster.index + 1,
                SizedOffset {
                    size: Size(0),
                    offset: Offset(0),
                },
            );
        }
        self.cluster_addresses[cluster.index] = SizedOffset {
            size: cluster.tail_size(),
            offset: cluster_offset,
        };
        self.open_cluster = None;
        Ok(())
    }

    fn get_open_cluster(&mut self, size: Size) -> Result<&mut ClusterCreator> {
        if let Some(cluster) = self.open_cluster.as_ref() {
            if cluster.is_full(size) {
                self.write_cluster()?;
            }
        }
        if self.open_cluster.is_none() {
            self.open_cluster();
        }
        Ok(self.open_cluster.as_mut().unwrap())
    }

    pub fn start(&mut self) -> Result<()> {
        self.file = Some(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&self.path)?,
        );
        self.file.as_mut().unwrap().seek(SeekFrom::Start(128))?;
        Ok(())
    }

    pub fn add_content(&mut self, content: &mut dyn Stream) -> Result<Idx<u32>> {
        let cluster = self.get_open_cluster(content.size())?;
        let entry_info = cluster.add_content(content)?;
        self.blob_addresses.push(entry_info);
        Ok(((self.blob_addresses.len() - 1) as u32).into())
    }

    pub fn finalize(&mut self) -> Result<PackInfo> {
        assert!(self.file.is_some());
        if let Some(cluster) = self.open_cluster.as_ref() {
            if !cluster.is_empty() {
                self.write_cluster()?;
            }
        }
        let file = self.file.as_mut().unwrap();
        let clusters_offset = file.tell();
        for address in &self.cluster_addresses {
            address.write(file)?;
        }
        let entries_offset = file.tell();
        for address in &self.blob_addresses {
            address.write(file)?;
        }
        let check_offset = file.tell();
        let pack_size: Size = (check_offset + 33).into();
        file.rewind()?;
        let header = ContentPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
            self.free_data,
            clusters_offset,
            (self.cluster_addresses.len() as u32).into(),
            entries_offset,
            (self.blob_addresses.len() as u32).into(),
        );
        header.write(file)?;
        file.rewind()?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_u8(1)?;
        file.write_all(hash.as_bytes())?;
        Ok(PackInfo {
            uuid: header.pack_header.uuid,
            pack_id: self.pack_id,
            free_data: FreeData::clone_from_slice(&[0; 103]),
            pack_size: pack_size.0,
            check_info: CheckInfo::new_blake3(hash.as_bytes()),
            pack_pos: PackPos::Path(self.path.to_str().unwrap().into()),
        })
    }
}
