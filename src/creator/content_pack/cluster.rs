use crate::bases::*;
use crate::common::{ClusterHeader, CompressionType, ContentInfo};

pub struct ClusterCreator {
    pub index: usize,
    compression: CompressionType,
    data: Vec<Reader>,
    offsets: Vec<usize>,
}

const CLUSTER_SIZE: Size = Size::new(1024 * 1024 * 4);
const MAX_BLOBS_PER_CLUSTER: usize = 0xFFF;

impl ClusterCreator {
    pub fn new(index: usize, compression: CompressionType) -> Self {
        ClusterCreator {
            index,
            compression,
            data: Vec::with_capacity(MAX_BLOBS_PER_CLUSTER),
            offsets: vec![],
        }
    }

    pub fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<Size> {
        let offset = stream.tell();
        let stream = match &self.compression {
            CompressionType::None => {
                for d in self.data.drain(..) {
                    std::io::copy(&mut d.create_stream_all(), stream)?;
                }
                stream
            }
            CompressionType::Lz4 => {
                let mut encoder = lz4::EncoderBuilder::new().level(16).build(stream)?;
                for d in self.data.drain(..) {
                    std::io::copy(&mut d.create_stream_all(), &mut encoder)?;
                }
                let (stream, err) = encoder.finish();
                err?;
                stream
            }
            CompressionType::Lzma => {
                let mut encoder = lzma::LzmaWriter::new_compressor(stream, 9)?;
                for d in self.data.drain(..) {
                    std::io::copy(&mut d.create_stream_all(), &mut encoder)?;
                }
                encoder.finish()?
            }
            CompressionType::Zstd => {
                let mut encoder = zstd::Encoder::new(stream, 19)?;
                encoder.multithread(8)?;
                encoder.include_contentsize(false)?;
                //encoder.long_distance_matching(true);
                for d in self.data.drain(..) {
                    std::io::copy(&mut d.create_stream_all(), &mut encoder)?;
                }
                encoder.finish()?
            }
        };
        Ok(stream.tell() - offset)
    }

    pub fn write_tail(&self, stream: &mut dyn OutStream, data_size: Size) -> IoResult<()> {
        let offset_size = needed_bytes(self.data_size().into_u64());
        let cluster_header = ClusterHeader::new(
            self.compression,
            offset_size,
            BlobCount::from(self.offsets.len() as u16),
        );
        cluster_header.write(stream)?;
        stream.write_sized(data_size.into_u64(), offset_size)?; // raw data size
        stream.write_sized(self.data_size().into_u64(), offset_size)?; // datasize
        for offset in &self.offsets[..self.offsets.len() - 1] {
            stream.write_sized(*offset as u64, offset_size)?;
        }
        Ok(())
    }

    pub fn tail_size(&self) -> Size {
        let mut size = 4;
        let size_byte = needed_bytes(self.data_size().into_u64());
        size += (1 + self.offsets.len()) * size_byte as usize;
        size.into()
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn data_size(&self) -> Size {
        Size::from(*self.offsets.last().unwrap_or(&0))
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

    pub fn add_content(&mut self, content: Reader) -> IoResult<ContentInfo> {
        assert!(self.offsets.len() < MAX_BLOBS_PER_CLUSTER);
        let idx = self.offsets.len() as u16;
        let new_offset = self.offsets.last().unwrap_or(&0) + content.size().into_usize();
        self.data.push(content);
        self.offsets.push(new_offset);
        Ok(ContentInfo::new(
            ClusterIdx::from(self.index as u32),
            BlobIdx::from(idx),
        ))
    }
}
