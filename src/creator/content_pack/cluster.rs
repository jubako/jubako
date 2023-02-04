use crate::bases::*;
use crate::common::{ClusterHeader, CompressionType, ContentInfo};
use crate::creator::private::WritableTell;

pub struct ClusterCreator {
    pub index: usize,
    pub compression: CompressionType,
    data: Vec<Reader>,
    offsets: Vec<usize>,
    raw_data_size: Late<usize>,
}

const CLUSTER_SIZE: Size = Size::new(1024 * 1024 * 4);
const MAX_BLOBS_PER_CLUSTER: usize = 0xFFF;

#[cfg(feature = "lz4")]
fn lz4_compress<'b>(
    data: &mut Vec<Reader>,
    stream: &'b mut dyn OutStream,
) -> Result<&'b mut dyn OutStream> {
    let mut encoder = lz4::EncoderBuilder::new().level(16).build(stream)?;
    for in_reader in data.drain(..) {
        std::io::copy(&mut in_reader.create_stream_all(), &mut encoder)?;
    }
    let (stream, err) = encoder.finish();
    err?;
    Ok(stream)
}

#[cfg(not(feature = "lz4"))]
#[allow(clippy::ptr_arg)]
fn lz4_compress<'b>(
    _data: &mut Vec<Reader>,
    _stream: &'b mut dyn OutStream,
) -> Result<&'b mut dyn OutStream> {
    Err("Lz4 compression is not supported by this configuration."
        .to_string()
        .into())
}

#[cfg(feature = "lzma")]
fn lzma_compress<'b>(
    data: &mut Vec<Reader>,
    stream: &'b mut dyn OutStream,
) -> Result<&'b mut dyn OutStream> {
    let mut encoder = lzma::LzmaWriter::new_compressor(stream, 9)?;
    for in_reader in data.drain(..) {
        std::io::copy(&mut in_reader.create_stream_all(), &mut encoder)?;
    }
    Ok(encoder.finish()?)
}

#[cfg(not(feature = "lzma"))]
#[allow(clippy::ptr_arg)]
fn lzma_compress<'b>(
    _data: &mut Vec<Reader>,
    _stream: &'b mut dyn OutStream,
) -> Result<&'b mut dyn OutStream> {
    Err("Lzma compression is not supported by this configuration."
        .to_string()
        .into())
}

#[cfg(feature = "zstd")]
fn zstd_compress<'b>(
    data: &mut Vec<Reader>,
    stream: &'b mut dyn OutStream,
) -> Result<&'b mut dyn OutStream> {
    let mut encoder = zstd::Encoder::new(stream, 19)?;
    encoder.multithread(8)?;
    encoder.include_contentsize(false)?;
    //encoder.long_distance_matching(true);
    for in_reader in data.drain(..) {
        std::io::copy(&mut in_reader.create_stream_all(), &mut encoder)?;
    }
    Ok(encoder.finish()?)
}

#[cfg(not(feature = "zstd"))]
#[allow(clippy::ptr_arg)]
fn zstd_compress<'b>(
    _data: &mut Vec<Reader>,
    _stream: &'b mut dyn OutStream,
) -> Result<&'b mut dyn OutStream> {
    Err("Zstd compression is not supported by this configuration."
        .to_string()
        .into())
}

impl ClusterCreator {
    pub fn new(index: usize, compression: CompressionType) -> Self {
        ClusterCreator {
            index,
            compression,
            data: Vec::with_capacity(MAX_BLOBS_PER_CLUSTER),
            offsets: vec![],
            raw_data_size: Default::default(),
        }
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

impl WritableTell for ClusterCreator {
    fn write_data(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        let offset = stream.tell();
        let stream = match &self.compression {
            CompressionType::None => {
                for d in self.data.drain(..) {
                    std::io::copy(&mut d.create_stream_all(), stream)?;
                }
                stream
            }
            CompressionType::Lz4 => lz4_compress(&mut self.data, stream)?,
            CompressionType::Lzma => lzma_compress(&mut self.data, stream)?,
            CompressionType::Zstd => zstd_compress(&mut self.data, stream)?,
        };
        self.raw_data_size
            .set((stream.tell() - offset).into_usize());
        Ok(())
    }

    fn write_tail(&mut self, stream: &mut dyn OutStream) -> Result<()> {
        let offset_size = needed_bytes(self.data_size().into_u64());
        let cluster_header = ClusterHeader::new(
            self.compression,
            offset_size,
            BlobCount::from(self.offsets.len() as u16),
        );
        cluster_header.write(stream)?;
        stream.write_sized(self.raw_data_size.get() as u64, offset_size)?; // raw data size
        stream.write_sized(self.data_size().into_u64(), offset_size)?; // datasize
        for offset in &self.offsets[..self.offsets.len() - 1] {
            stream.write_sized(*offset as u64, offset_size)?;
        }
        Ok(())
    }
}
