use crate::bases::*;
use crate::common::{ClusterHeader, CompressionType, ContentInfo};
use std::io::Cursor;
use std::io::Read;

pub struct ClusterCreator {
    pub index: usize,
    compression: CompressionType,
    data: Vec<u8>,
    offsets: Vec<usize>,
}

const CLUSTER_SIZE: Size = Size::new(1024 * 1024 * 4);
const MAX_BLOBS_PER_CLUSTER: usize = 0xFFF;

impl ClusterCreator {
    pub fn new(index: usize, compression: CompressionType) -> Self {
        ClusterCreator {
            index,
            compression,
            data: Vec::with_capacity(CLUSTER_SIZE.into_usize()),
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
        let cluster_header = ClusterHeader::new(
            self.compression,
            offset_size as u8,
            BlobCount::from(self.offsets.len() as u16),
        );
        cluster_header.write(stream)?;
        stream.write_sized(data_size.into_u64(), offset_size)?; // raw data size
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

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn data_size(&self) -> Size {
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

    pub fn add_content(&mut self, content: &mut Stream) -> IoResult<ContentInfo> {
        assert!(self.offsets.len() < MAX_BLOBS_PER_CLUSTER);
        let idx = self.offsets.len() as u16;
        content.read_to_end(&mut self.data)?;
        self.offsets.push(self.data.len());
        Ok(ContentInfo::new(
            ClusterIdx::from(self.index as u32),
            BlobIdx::from(idx),
        ))
    }
}
