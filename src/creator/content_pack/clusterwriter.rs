use super::cluster::ClusterCreator;
use crate::bases::*;
use crate::common::{ClusterHeader, CompressionType};
use std::fs::File;
use std::io::Write;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread::{spawn, JoinHandle};

#[cfg(feature = "lz4")]
fn lz4_compress<'b>(
    data: &mut Vec<Reader>,
    stream: &'b mut dyn OutStream,
) -> Result<&'b mut dyn OutStream> {
    let mut encoder = lz4::EncoderBuilder::new().level(16).build(stream)?;
    for in_reader in data.drain(..) {
        std::io::copy(&mut in_reader.create_flux_all(), &mut encoder)?;
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
        std::io::copy(&mut in_reader.create_flux_all(), &mut encoder)?;
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
        std::io::copy(&mut in_reader.create_flux_all(), &mut encoder)?;
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

pub struct ClusterCompressor {
    compression: CompressionType,
    input: spmc::Receiver<ClusterCreator>,
    output: mpsc::Sender<WriteTask>,
    nb_cluster_in_queue: Arc<(Mutex<usize>, Condvar)>,
}

impl ClusterCompressor {
    pub fn new(
        compression: CompressionType,
        input: spmc::Receiver<ClusterCreator>,
        output: mpsc::Sender<WriteTask>,
        nb_cluster_in_queue: Arc<(Mutex<usize>, Condvar)>,
    ) -> Self {
        Self {
            compression,
            input,
            output,
            nb_cluster_in_queue,
        }
    }

    fn write_cluster_data(
        &mut self,
        cluster: &mut ClusterCreator,
        outstream: &mut dyn OutStream,
    ) -> Result<()> {
        match &self.compression {
            CompressionType::None => unreachable!(),
            CompressionType::Lz4 => lz4_compress(&mut cluster.data, outstream)?,
            CompressionType::Lzma => lzma_compress(&mut cluster.data, outstream)?,
            CompressionType::Zstd => zstd_compress(&mut cluster.data, outstream)?,
        };
        Ok(())
    }

    fn write_cluster_tail(
        &mut self,
        cluster: &mut ClusterCreator,
        raw_data_size: Size,
        outstream: &mut dyn OutStream,
    ) -> Result<()> {
        let offset_size = needed_bytes(cluster.data_size().into_u64());
        let cluster_header = ClusterHeader::new(
            self.compression,
            offset_size,
            BlobCount::from(cluster.offsets.len() as u16),
        );
        cluster_header.write(outstream)?;
        outstream.write_usized(raw_data_size.into_u64(), offset_size)?; // raw data size
        outstream.write_usized(cluster.data_size().into_u64(), offset_size)?; // datasize
        for offset in &cluster.offsets[..cluster.offsets.len() - 1] {
            outstream.write_usized(*offset as u64, offset_size)?;
        }
        Ok(())
    }

    pub fn compress_cluster(
        &mut self,
        mut cluster: ClusterCreator,
        outstream: &mut dyn OutStream,
    ) -> Result<SizedOffset> {
        println!("Compress cluster {}", cluster.index());
        self.write_cluster_data(&mut cluster, outstream)?;
        let tail_offset = outstream.tell();
        self.write_cluster_tail(&mut cluster, tail_offset.into(), outstream)?;
        let tail_size = outstream.tell() - tail_offset;
        Ok(SizedOffset {
            size: tail_size,
            offset: tail_offset,
        })
    }

    pub fn run(mut self) -> Result<()> {
        while let Ok(cluster) = self.input.recv() {
            let mut data = Vec::<u8>::with_capacity(1024 * 1024);
            let mut cursor = std::io::Cursor::new(&mut data);
            let cluster_idx = cluster.index();
            let sized_offset = self.compress_cluster(cluster, &mut cursor)?;
            self.output
                .send(WriteTask::Compressed(data, sized_offset, cluster_idx))
                .unwrap();
            let (count, cvar) = &*self.nb_cluster_in_queue;
            let mut count = count.lock().unwrap();
            *count -= 1;
            cvar.notify_one();
        }
        drop(self.output);
        Ok(())
    }
}

pub enum WriteTask {
    Cluster(ClusterCreator),
    Compressed(Vec<u8>, SizedOffset, usize),
}

impl From<ClusterCreator> for WriteTask {
    fn from(c: ClusterCreator) -> Self {
        Self::Cluster(c)
    }
}

pub struct ClusterWriter {
    cluster_addresses: Vec<Late<SizedOffset>>,
    file: File,
    input: mpsc::Receiver<WriteTask>,
}

impl ClusterWriter {
    pub fn new(file: File, input: mpsc::Receiver<WriteTask>) -> Self {
        Self {
            cluster_addresses: vec![],
            file,
            input,
        }
    }

    fn write_cluster_data(&mut self, cluster: &mut ClusterCreator) -> Result<()> {
        for d in cluster.data.drain(..) {
            std::io::copy(&mut d.create_flux_all(), &mut self.file)?;
        }
        Ok(())
    }

    fn write_cluster_tail(
        &mut self,
        cluster: &mut ClusterCreator,
        raw_data_size: Size,
    ) -> Result<()> {
        let offset_size = needed_bytes(cluster.data_size().into_u64());
        let cluster_header = ClusterHeader::new(
            CompressionType::None,
            offset_size,
            BlobCount::from(cluster.offsets.len() as u16),
        );
        cluster_header.write(&mut self.file)?;
        self.file
            .write_usized(raw_data_size.into_u64(), offset_size)?; // raw data size
        self.file
            .write_usized(cluster.data_size().into_u64(), offset_size)?; // datasize
        for offset in &cluster.offsets[..cluster.offsets.len() - 1] {
            self.file.write_usized(*offset as u64, offset_size)?;
        }
        Ok(())
    }

    fn write_cluster(&mut self, mut cluster: ClusterCreator) -> Result<SizedOffset> {
        println!("Write cluster {}", cluster.index());
        let start_offset = self.file.tell();
        self.write_cluster_data(&mut cluster)?;
        let tail_offset = self.file.tell();
        self.write_cluster_tail(&mut cluster, tail_offset - start_offset)?;
        let tail_size = self.file.tell() - tail_offset;
        Ok(SizedOffset {
            size: tail_size,
            offset: tail_offset,
        })
    }

    fn write_data(&mut self, data: &[u8]) -> Result<Offset> {
        let offset = self.file.tell();
        self.file.write_all(data)?;
        Ok(offset)
    }

    pub fn run(mut self) -> Result<(File, Vec<Late<SizedOffset>>)> {
        while let Ok(task) = self.input.recv() {
            let (sized_offset, idx) = match task {
                WriteTask::Cluster(cluster) => {
                    let cluster_idx = cluster.index();
                    let sized_offset = self.write_cluster(cluster)?;
                    (sized_offset, cluster_idx)
                }
                WriteTask::Compressed(data, mut sized_offset, idx) => {
                    let offset = self.write_data(&data)?;
                    sized_offset.offset += offset;
                    (sized_offset, idx)
                }
            };
            if self.cluster_addresses.len() <= idx {
                self.cluster_addresses.resize(idx + 1, Default::default());
            }
            self.cluster_addresses[idx].set(sized_offset);
        }
        Ok((self.file, self.cluster_addresses))
    }
}

pub struct ClusterWriterProxy {
    worker_threads: Vec<JoinHandle<Result<()>>>,
    thread_handle: JoinHandle<Result<(File, Vec<Late<SizedOffset>>)>>,
    dispatch_tx: spmc::Sender<ClusterCreator>,
    fusion_tx: mpsc::Sender<WriteTask>,
    nb_cluster_in_queue: Arc<(Mutex<usize>, Condvar)>,
    max_queue_size: usize,
    compression: CompressionType,
}

impl ClusterWriterProxy {
    pub fn new(file: File, compression: CompressionType, nb_thread: usize) -> Self {
        let (dispatch_tx, dispatch_rx) = spmc::channel();
        let (fusion_tx, fusion_rx) = mpsc::channel();

        let nb_cluster_in_queue = Arc::new((Mutex::new(0), Condvar::new()));

        let worker_threads = (0..nb_thread)
            .map(|_| {
                let dispatch_rx = dispatch_rx.clone();
                let fusion_tx = fusion_tx.clone();
                let nb_cluster_in_queue = Arc::clone(&nb_cluster_in_queue);
                spawn(move || {
                    let worker = ClusterCompressor::new(
                        compression,
                        dispatch_rx,
                        fusion_tx,
                        nb_cluster_in_queue,
                    );
                    worker.run()
                })
            })
            .collect();

        let thread_handle = spawn(move || {
            let writer = ClusterWriter::new(file, fusion_rx);
            writer.run()
        });
        Self {
            thread_handle,
            worker_threads,
            dispatch_tx,
            fusion_tx,
            nb_cluster_in_queue,
            max_queue_size: nb_thread * 2,
            compression,
        }
    }

    pub fn write_cluster(&mut self, cluster: ClusterCreator, compressed: bool) {
        if compressed && self.compression != CompressionType::None {
            let (count, cvar) = &*self.nb_cluster_in_queue;
            let mut count = cvar
                .wait_while(count.lock().unwrap(), |c| *c >= self.max_queue_size)
                .unwrap();
            *count += 1;
            self.dispatch_tx.send(cluster).unwrap();
        } else {
            self.fusion_tx.send(cluster.into()).unwrap();
        }
    }

    pub fn finalize(self) -> Result<(File, Vec<Late<SizedOffset>>)> {
        drop(self.dispatch_tx);
        drop(self.fusion_tx);
        for thread in self.worker_threads {
            thread.join().unwrap()?;
        }
        self.thread_handle.join().unwrap()
    }
}
