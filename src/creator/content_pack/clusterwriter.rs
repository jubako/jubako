use dropout::Dropper;

use super::cluster::ClusterCreator;
use super::Progress;
use crate::bases::*;
use crate::common::ClusterHeader;
use crate::creator::{Compression, InputReader, MaybeFileReader};
use std::io::{BufWriter, Write};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread::JoinHandle;

#[inline(always)]
fn spawn<F, T>(name: &str, f: F) -> std::thread::JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .name(name.into())
        .spawn(f)
        .expect("Success to launch thread")
}

type InputData = Vec<Box<dyn InputReader>>;

#[cfg(feature = "lz4")]
fn lz4_compress<'b>(
    data: &mut InputData,
    stream: &'b mut dyn OutStream,
    level: u32,
) -> std::io::Result<&'b mut dyn OutStream> {
    let mut encoder = lz4::EncoderBuilder::new()
        .level(level)
        .block_size(lz4::BlockSize::Max4MB)
        .block_mode(lz4::BlockMode::Linked)
        .checksum(lz4::ContentChecksum::NoChecksum)
        .build(stream)?;
    for mut in_reader in data.drain(..) {
        std::io::copy(&mut in_reader, &mut encoder)?;
    }
    let (stream, err) = encoder.finish();
    err?;
    Ok(stream)
}

#[cfg(feature = "lzma")]
fn lzma_compress<'b>(
    data: &mut InputData,
    stream: &'b mut dyn OutStream,
    level: u32,
) -> std::io::Result<&'b mut dyn OutStream> {
    let mut encoder = liblzma::write::XzEncoder::new_stream(
        stream,
        liblzma::stream::Stream::new_lzma_encoder(&liblzma::stream::LzmaOptions::new_preset(
            level,
        )?)?,
    );
    for mut in_reader in data.drain(..) {
        std::io::copy(&mut in_reader, &mut encoder)?;
    }
    encoder.finish()
}

#[cfg(feature = "zstd")]
fn zstd_compress<'b>(
    data: &mut InputData,
    stream: &'b mut dyn OutStream,
    level: i32,
) -> std::io::Result<&'b mut dyn OutStream> {
    let mut encoder = zstd::Encoder::new(stream, level)?;
    encoder.include_contentsize(false)?;
    encoder.include_checksum(false)?;
    encoder.window_log(23)?;
    for mut in_reader in data.drain(..) {
        std::io::copy(&mut in_reader, &mut encoder)?;
    }
    encoder.finish()
}

struct ClusterCompressor {
    compression: Compression,
    input: spmc::Receiver<ClusterCreator>,
    output: mpsc::Sender<WriteTask>,
    nb_cluster_in_queue: Arc<(Mutex<usize>, Condvar)>,
    progress: Arc<dyn Progress>,
}
fn serialize_cluster_tail(
    compression: Compression,
    cluster: &ClusterCreator,
    raw_data_size: Size,
    ser: &mut Serializer,
) -> std::io::Result<()> {
    let offset_size = needed_bytes(cluster.data_size().into_u64());
    let cluster_header = ClusterHeader::new(
        compression.into(),
        offset_size,
        BlobCount::from(cluster.offsets.len() as u16),
    );
    cluster_header.serialize(ser)?;
    ser.write_usized(raw_data_size.into_u64(), offset_size)?; // raw data size
    ser.write_usized(cluster.data_size().into_u64(), offset_size)?; // datasize
    for offset in &cluster.offsets[..cluster.offsets.len() - 1] {
        ser.write_usized(*offset, offset_size)?;
    }
    Ok(())
}

impl ClusterCompressor {
    pub fn new(
        compression: Compression,
        input: spmc::Receiver<ClusterCreator>,
        output: mpsc::Sender<WriteTask>,
        nb_cluster_in_queue: Arc<(Mutex<usize>, Condvar)>,
        progress: Arc<dyn Progress>,
    ) -> Self {
        Self {
            compression,
            input,
            output,
            nb_cluster_in_queue,
            progress,
        }
    }

    fn write_cluster_data(
        &mut self,
        cluster: &mut ClusterCreator,
        outstream: &mut dyn OutStream,
    ) -> std::io::Result<()> {
        match &self.compression {
            Compression::None => unreachable!(),
            #[cfg(feature = "lz4")]
            Compression::Lz4(level) => lz4_compress(&mut cluster.data, outstream, level.get())?,
            #[cfg(feature = "lzma")]
            Compression::Lzma(level) => lzma_compress(&mut cluster.data, outstream, level.get())?,
            #[cfg(feature = "zstd")]
            Compression::Zstd(level) => zstd_compress(&mut cluster.data, outstream, level.get())?,
        };
        Ok(())
    }

    pub fn compress_cluster(
        &mut self,
        mut cluster: ClusterCreator,
        outstream: &mut dyn OutStream,
    ) -> std::io::Result<SizedOffset> {
        self.progress.handle_cluster(cluster.index.into(), true);
        let data_offset = outstream.tell();
        self.write_cluster_data(&mut cluster, outstream)?;
        let tail_offset = outstream.tell();
        let mut serializer = Serializer::new(BlockCheck::Crc32);
        serialize_cluster_tail(
            self.compression,
            &cluster,
            tail_offset - data_offset,
            &mut serializer,
        )?;
        let tail_size = outstream.write_serializer(serializer)?.into();
        Ok(SizedOffset {
            size: tail_size,
            offset: tail_offset,
        })
    }

    pub fn run(mut self) -> std::io::Result<()> {
        while let Ok(cluster) = self.input.recv() {
            //[TODO] Avoid allocation. Reuse the data once it is written ?
            let mut data = Vec::<u8>::with_capacity(1024 * 1024);
            let mut cursor = std::io::Cursor::new(&mut data);
            let cluster_idx = cluster.index;
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

enum WriteTask {
    Cluster(ClusterCreator),
    Compressed(Vec<u8>, SizedOffset, ClusterIdx),
}

impl From<ClusterCreator> for WriteTask {
    fn from(c: ClusterCreator) -> Self {
        Self::Cluster(c)
    }
}

struct ClusterWriter<O: OutStream> {
    cluster_addresses: Vec<Late<SizedOffset>>,
    file: BufWriter<O>,
    input: mpsc::Receiver<WriteTask>,
    dropper: Dropper<MaybeFileReader>,
    progress: Arc<dyn Progress>,
}

impl<O> ClusterWriter<O>
where
    O: OutStream,
{
    pub fn new(file: O, input: mpsc::Receiver<WriteTask>, progress: Arc<dyn Progress>) -> Self {
        Self {
            cluster_addresses: vec![],
            file: BufWriter::new(file),
            input,
            dropper: Dropper::new(),
            progress,
        }
    }

    fn write_cluster_data(&mut self, cluster_data: InputData) -> std::io::Result<u64> {
        let mut copied = 0;
        for d in cluster_data.into_iter() {
            let (read, to_drop) = self.file.copy(d)?;
            copied += read;
            self.dropper.dropout(to_drop);
        }
        Ok(copied)
    }

    fn write_cluster(&mut self, mut cluster: ClusterCreator) -> std::io::Result<SizedOffset> {
        self.progress
            .handle_cluster(cluster.index.into_u32(), false);
        let start_offset = self.file.tell();
        let written = self.write_cluster_data(cluster.data.split_off(0))?;
        assert_eq!(written, cluster.data_size().into_u64());
        let tail_offset = self.file.tell();
        let mut serializer = Serializer::new(BlockCheck::Crc32);
        serialize_cluster_tail(
            Compression::None,
            &cluster,
            tail_offset - start_offset,
            &mut serializer,
        )?;
        let tail_size = self.file.write_serializer(serializer)?.into();
        Ok(SizedOffset {
            size: tail_size,
            offset: tail_offset,
        })
    }

    fn write_data(&mut self, data: &[u8]) -> std::io::Result<Offset> {
        let offset = self.file.tell();
        self.file.write_all(data)?;
        Ok(offset)
    }

    pub fn run(mut self) -> std::io::Result<(O, Vec<Late<SizedOffset>>)> {
        while let Ok(task) = self.input.recv() {
            let (sized_offset, idx) = match task {
                WriteTask::Cluster(cluster) => {
                    let cluster_idx = cluster.index;
                    let sized_offset = self.write_cluster(cluster)?;
                    (sized_offset, cluster_idx)
                }
                WriteTask::Compressed(data, mut sized_offset, idx) => {
                    let offset = self.write_data(&data)?;
                    sized_offset.offset += offset;
                    (sized_offset, idx)
                }
            };
            self.progress.handle_cluster_written(idx.into_u32());
            let idx = idx.into_usize();
            if self.cluster_addresses.len() <= idx {
                self.cluster_addresses.resize(idx + 1, Default::default());
            }
            self.cluster_addresses[idx].set(sized_offset);
        }
        Ok((
            self.file.into_inner().map_err(|e| e.into_error())?,
            self.cluster_addresses,
        ))
    }
}

pub(super) struct ClusterWriterProxy<O: OutStream> {
    worker_threads: Vec<JoinHandle<std::io::Result<()>>>,
    thread_handle: JoinHandle<std::io::Result<(O, Vec<Late<SizedOffset>>)>>,
    dispatch_tx: spmc::Sender<ClusterCreator>,
    fusion_tx: mpsc::Sender<WriteTask>, // FIXME: Should we use a `mpsc::SyncSender` instead ?
    nb_cluster_in_queue: Arc<(Mutex<usize>, Condvar)>,
    max_queue_size: usize,
    compression: Compression,
}

impl<O: OutStream + 'static> ClusterWriterProxy<O> {
    pub fn new(
        file: O,
        compression: Compression,
        nb_thread: usize,
        progress: Arc<dyn Progress>,
    ) -> Self {
        let (dispatch_tx, dispatch_rx) = spmc::channel();
        let (fusion_tx, fusion_rx) = mpsc::channel();

        let nb_cluster_in_queue = Arc::new((Mutex::new(0), Condvar::new()));

        let worker_threads = (0..nb_thread)
            .map(|idx| {
                let dispatch_rx = dispatch_rx.clone();
                let fusion_tx = fusion_tx.clone();
                let nb_cluster_in_queue = Arc::clone(&nb_cluster_in_queue);
                let progress = Arc::clone(&progress);
                spawn(&format!("ClusterComp {idx}"), move || {
                    let worker = ClusterCompressor::new(
                        compression,
                        dispatch_rx,
                        fusion_tx,
                        nb_cluster_in_queue,
                        progress,
                    );
                    worker.run()
                })
            })
            .collect();

        let thread_handle = spawn("Cluster writer", move || {
            let writer = ClusterWriter::new(file, fusion_rx, progress);
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

    pub fn write_cluster(
        &mut self,
        cluster: ClusterCreator,
        compressed: bool,
    ) -> std::io::Result<()> {
        let should_compress = if let Compression::None = self.compression {
            false
        } else {
            compressed
        };
        if should_compress {
            let (count, cvar) = &*self.nb_cluster_in_queue;
            let mut count = cvar
                .wait_while(count.lock().unwrap(), |c| *c >= self.max_queue_size)
                .unwrap();
            *count += 1;
            self.dispatch_tx
                .send(cluster)
                .expect("Receiver should not be closed");
        } else {
            self.fusion_tx
                .send(cluster.into())
                .expect("Receiver should not be closed");
        }
        Ok(())
    }

    pub fn finalize(self) -> std::io::Result<(O, Vec<Late<SizedOffset>>)> {
        drop(self.dispatch_tx);
        drop(self.fusion_tx);
        for thread in self.worker_threads {
            thread.join().unwrap()?;
        }
        self.thread_handle.join().unwrap()
    }
}
