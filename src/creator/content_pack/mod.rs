mod cluster;
mod clusterwriter;

use crate::bases::*;
use crate::common::{CompressionType, ContentInfo, ContentPackHeader, PackHeaderInfo};
use crate::creator::{Embedded, PackData};
use cluster::ClusterCreator;
use clusterwriter::ClusterWriterProxy;
use std::cell::Cell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn shannon_entropy(data: &Reader) -> Result<f32> {
    let mut entropy = 0.0;
    let mut counts = [0; 256];
    let size = std::cmp::min(1024, data.size().into_usize());
    let mut data = data.create_flux_all();

    for _ in 0..size {
        counts[data.read_u8()? as usize] += 1;
    }

    for &count in &counts {
        if count == 0 {
            continue;
        }

        let p: f32 = (count as f32) / (size as f32);
        entropy -= p * p.log(2.0);
    }

    Ok(entropy)
}

pub trait Progress: Send + Sync {
    fn new_cluster(&self, _cluster_idx: u32, _compressed: bool) {}
    fn handle_cluster(&self, _cluster_idx: u32, _compressed: bool) {}
    fn content_added(&self, _size: Size) {}
}

impl Progress for () {}

pub struct ContentPackCreator {
    app_vendor_id: u32,
    pack_id: PackId,
    free_data: FreeData40,
    content_infos: Vec<ContentInfo>,
    raw_open_cluster: Option<ClusterCreator>,
    comp_open_cluster: Option<ClusterCreator>,
    next_cluster_id: Cell<u32>,
    cluster_writer: ClusterWriterProxy,
    progress: Arc<dyn Progress>,
}

macro_rules! open_cluster_ref {
    ($self:expr, $comp: expr) => {
        if $comp {
            &$self.comp_open_cluster
        } else {
            &$self.raw_open_cluster
        }
    };
    (mut $self:expr, $comp: expr) => {
        if $comp {
            &mut $self.comp_open_cluster
        } else {
            &mut $self.raw_open_cluster
        }
    };
}

impl ContentPackCreator {
    pub fn new<P: AsRef<Path>>(
        path: P,
        pack_id: PackId,
        app_vendor_id: u32,
        free_data: FreeData40,
        compression: CompressionType,
    ) -> Result<Self> {
        Self::new_with_progress(
            path,
            pack_id,
            app_vendor_id,
            free_data,
            compression,
            Arc::new(()),
        )
    }

    pub fn new_with_progress<P: AsRef<Path>>(
        path: P,
        pack_id: PackId,
        app_vendor_id: u32,
        free_data: FreeData40,
        compression: CompressionType,
        progress: Arc<dyn Progress>,
    ) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        Self::new_from_file_with_progress(
            file,
            pack_id,
            app_vendor_id,
            free_data,
            compression,
            progress,
        )
    }

    pub fn new_from_file(
        file: File,
        pack_id: PackId,
        app_vendor_id: u32,
        free_data: FreeData40,
        compression: CompressionType,
    ) -> Result<Self> {
        Self::new_from_file_with_progress(
            file,
            pack_id,
            app_vendor_id,
            free_data,
            compression,
            Arc::new(()),
        )
    }

    pub fn new_from_file_with_progress(
        mut file: File,
        pack_id: PackId,
        app_vendor_id: u32,
        free_data: FreeData40,
        compression: CompressionType,
        progress: Arc<dyn Progress>,
    ) -> Result<Self> {
        file.seek(SeekFrom::Start(128))?;
        let nb_threads = std::thread::available_parallelism()
            .unwrap_or(8.try_into().unwrap())
            .get();
        let cluster_writer =
            ClusterWriterProxy::new(file, compression, nb_threads, Arc::clone(&progress));
        Ok(Self {
            app_vendor_id,
            pack_id,
            free_data,
            content_infos: vec![],
            raw_open_cluster: None,
            comp_open_cluster: None,
            next_cluster_id: Cell::new(0),
            cluster_writer,
            progress,
        })
    }

    fn open_cluster(&self, compressed: bool) -> ClusterCreator {
        let cluster_id = self.next_cluster_id.replace(self.next_cluster_id.get() + 1);
        self.progress.new_cluster(cluster_id, compressed);
        ClusterCreator::new(cluster_id.into())
    }

    fn get_open_cluster<'s>(&'s mut self, content: &Reader) -> Result<&'s mut ClusterCreator> {
        let entropy = shannon_entropy(content)?;
        let compress_content = entropy <= 6.0;
        // Let's get raw cluster
        if let Some(cluster) = self.cluster_to_close(content.size(), compress_content) {
            self.cluster_writer.write_cluster(cluster, compress_content);
        }
        Ok(open_cluster_ref!(mut self, compress_content)
            .as_mut()
            .unwrap())
    }

    fn cluster_to_close(&mut self, size: Size, compressed: bool) -> Option<ClusterCreator> {
        if let Some(cluster) = open_cluster_ref!(self, compressed).as_ref() {
            if cluster.is_full(size) {
                let new_cluster = self.open_cluster(compressed);
                open_cluster_ref!(mut self, compressed).replace(new_cluster)
            } else {
                None
            }
        } else {
            let new_cluster = self.open_cluster(compressed);
            open_cluster_ref!(mut self, compressed).replace(new_cluster)
        }
    }

    pub fn add_content(&mut self, content: Reader) -> Result<ContentIdx> {
        self.progress.content_added(content.size());
        let cluster = self.get_open_cluster(&content)?;
        let content_info = cluster.add_content(content)?;
        self.content_infos.push(content_info);
        Ok(((self.content_infos.len() - 1) as u32).into())
    }

    pub fn finalize(mut self, path: Option<PathBuf>) -> Result<PackData> {
        if let Some(cluster) = self.raw_open_cluster.take() {
            if !cluster.is_empty() {
                self.cluster_writer.write_cluster(cluster, false);
            }
        }
        if let Some(cluster) = self.comp_open_cluster.take() {
            if !cluster.is_empty() {
                self.cluster_writer.write_cluster(cluster, true);
            }
        }

        let (mut file, cluster_addresses) = self.cluster_writer.finalize()?;
        let clusters_offset = file.tell();
        let nb_clusters = cluster_addresses.len();
        for address in cluster_addresses {
            address.get().write(&mut file)?;
        }
        let content_infos_offset = file.tell();
        for content_info in &self.content_infos {
            content_info.write(&mut file)?;
        }
        let check_offset = file.tell();
        let pack_size: Size = (check_offset + 33 + 64).into();
        file.rewind()?;
        let header = ContentPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
            self.free_data,
            clusters_offset,
            (nb_clusters as u32).into(),
            content_infos_offset,
            (self.content_infos.len() as u32).into(),
        );
        header.write(&mut file)?;
        file.rewind()?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_u8(1)?;
        file.write_all(hash.as_bytes())?;

        file.rewind()?;
        let mut tail_buffer = [0u8; 64];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        file.rewind()?;
        Ok(PackData {
            uuid: header.pack_header.uuid,
            pack_id: self.pack_id,
            free_data: FreeData103::clone_from_slice(&[0; 103]),
            reader: FileSource::new(file)?.into(),
            check_info_pos: check_offset,
            embedded: match path {
                None => Embedded::Yes,
                Some(p) => Embedded::No(p),
            },
        })
    }
}
