use super::cluster::ClusterCreator;
use crate::bases::*;
use crate::common::{ClusterHeader, CompressionType};
use std::fs::File;
use std::thread::{spawn, JoinHandle};

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

pub struct ClusterWriter {
    cluster_addresses: Vec<SizedOffset>,
    pub compression: CompressionType,
    file: File,
    input: spmc::Receiver<(ClusterCreator, bool)>,
}

impl ClusterWriter {
    pub fn new(
        file: File,
        compression: CompressionType,
        input: spmc::Receiver<(ClusterCreator, bool)>,
    ) -> Self {
        Self {
            cluster_addresses: vec![],
            compression,
            file,
            input,
        }
    }

    fn write_cluster_data(&mut self, cluster: &mut ClusterCreator, compressed: bool) -> Result<()> {
        if compressed && self.compression != CompressionType::None {
            match &self.compression {
                CompressionType::None => unreachable!(),
                CompressionType::Lz4 => lz4_compress(&mut cluster.data, &mut self.file)?,
                CompressionType::Lzma => lzma_compress(&mut cluster.data, &mut self.file)?,
                CompressionType::Zstd => zstd_compress(&mut cluster.data, &mut self.file)?,
            };
        } else {
            for d in cluster.data.drain(..) {
                std::io::copy(&mut d.create_stream_all(), &mut self.file)?;
            }
        };
        Ok(())
    }

    fn write_cluster_tail(
        &mut self,
        cluster: &mut ClusterCreator,
        compressed: bool,
        raw_data_size: Size,
    ) -> Result<()> {
        let offset_size = needed_bytes(cluster.data_size().into_u64());
        let cluster_header = ClusterHeader::new(
            if compressed { self.compression} else { CompressionType::None },
            offset_size,
            BlobCount::from(cluster.offsets.len() as u16),
        );
        cluster_header.write(&mut self.file)?;
        self.file
            .write_sized(raw_data_size.into_u64(), offset_size)?; // raw data size
        self.file
            .write_sized(cluster.data_size().into_u64(), offset_size)?; // datasize
        for offset in &cluster.offsets[..cluster.offsets.len() - 1] {
            self.file.write_sized(*offset as u64, offset_size)?;
        }
        Ok(())
    }

    pub fn write_cluster(&mut self, mut cluster: ClusterCreator, compressed: bool) -> Result<()> {
        println!("Write cluster {}", cluster.index());
        let start_offset = self.file.tell();
        self.write_cluster_data(&mut cluster, compressed)?;
        let tail_offset = self.file.tell();
        self.write_cluster_tail(&mut cluster, compressed, tail_offset - start_offset)?;
        let tail_size = self.file.tell() - tail_offset;
        if self.cluster_addresses.len() <= cluster.index() {
            self.cluster_addresses.resize(
                cluster.index() + 1,
                Default::default()
            );
        }
        self.cluster_addresses[cluster.index()] = SizedOffset {
            size: tail_size,
            offset: tail_offset,
        };
        Ok(())
    }

    pub fn run(mut self) -> Result<(File, Vec<SizedOffset>)> {
        while let Ok((cluster, compressed)) = self.input.recv() {
            self.write_cluster(cluster, compressed)?;
        }
        Ok((self.file, self.cluster_addresses))
    }
}

pub struct ClusterWriterProxy {
    thread_handle: JoinHandle<Result<(File, Vec<SizedOffset>)>>,
    dispatch_tx: spmc::Sender<(ClusterCreator, bool)>,
}

impl ClusterWriterProxy {
    pub fn new(file: File, compression: CompressionType) -> Self {
        let (dispatch_tx, dispatch_rx) = spmc::channel();
        let thread_handle = spawn(move || {
            let writer = ClusterWriter::new(file, compression, dispatch_rx);
            writer.run()
        });
        Self {
            thread_handle,
            dispatch_tx,
        }
    }

    pub fn write_cluster(&mut self, cluster: ClusterCreator, compressed: bool) {
        self.dispatch_tx.send((cluster, compressed)).unwrap()
    }

    pub fn finalize(self) -> Result<(File, Vec<SizedOffset>)> {
        drop(self.dispatch_tx);
        self.thread_handle.join().unwrap()
    }
}
