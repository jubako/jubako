use super::cluster::ClusterCreator;
use super::clusterwriter::ClusterWriterProxy;
use super::{CompHint, ContentAdder, Progress};
use crate::bases::*;
use crate::common::{
    CheckInfo, CheckKind, ContentAddress, ContentInfo, ContentPackHeader, PackHeader,
    PackHeaderInfo, PackKind,
};
use crate::creator::{Compression, InputReader, NamedFile, PackData, PackRecipient};
use std::cell::Cell;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Arc;

use log::info;

fn shannon_entropy(data: &[u8]) -> Result<f32> {
    let mut entropy = 0.0;
    let mut counts = [0; 256];

    for byte in data {
        counts[*byte as usize] += 1;
    }

    for &count in &counts {
        if count == 0 {
            continue;
        }

        let p: f32 = (count as f32) / (data.len() as f32);
        entropy -= p * p.log(2.0);
    }

    Ok(entropy)
}

pub struct ContentPackCreator<O: PackRecipient + ?Sized> {
    app_vendor_id: VendorId,
    pack_id: PackId,
    free_data: PackFreeData,
    content_infos: Vec<ContentInfo>,
    raw_open_cluster: Option<ClusterCreator>,
    comp_open_cluster: Option<ClusterCreator>,
    next_cluster_id: Cell<u32>,
    cluster_writer: ClusterWriterProxy<Box<O>>,
    progress: Arc<dyn Progress>,
    compression: Compression,
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

impl ContentPackCreator<NamedFile> {
    pub fn new<P: AsRef<Path>>(
        path: P,
        pack_id: PackId,
        app_vendor_id: VendorId,
        free_data: PackFreeData,
        compression: Compression,
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
        app_vendor_id: VendorId,
        free_data: PackFreeData,
        compression: Compression,
        progress: Arc<dyn Progress>,
    ) -> Result<Self> {
        let file = NamedFile::new(path)?;
        Self::new_from_output_with_progress(
            file,
            pack_id,
            app_vendor_id,
            free_data,
            compression,
            progress,
        )
    }
}

impl<O: PackRecipient + 'static + ?Sized> ContentPackCreator<O> {
    pub fn new_from_output(
        file: Box<O>,
        pack_id: PackId,
        app_vendor_id: VendorId,
        free_data: PackFreeData,
        compression: Compression,
    ) -> Result<Self> {
        Self::new_from_output_with_progress(
            file,
            pack_id,
            app_vendor_id,
            free_data,
            compression,
            Arc::new(()),
        )
    }

    pub fn new_from_output_with_progress(
        mut file: Box<O>,
        pack_id: PackId,
        app_vendor_id: VendorId,
        free_data: PackFreeData,
        compression: Compression,
        progress: Arc<dyn Progress>,
    ) -> Result<Self> {
        file.seek(SeekFrom::Start(
            PackHeader::BLOCK_SIZE as u64 + ContentPackHeader::BLOCK_SIZE as u64,
        ))?;
        let nb_threads = std::cmp::max(
            std::thread::available_parallelism()
                .unwrap_or(8.try_into().unwrap())
                .get(),
            2,
        ) - 1;
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
            compression,
        })
    }

    fn open_cluster(&self, compressed: bool) -> ClusterCreator {
        let cluster_id = self.next_cluster_id.replace(self.next_cluster_id.get() + 1);
        self.progress.new_cluster(cluster_id, compressed);
        ClusterCreator::new(cluster_id.into(), compressed)
    }

    fn get_open_cluster(
        &mut self,
        compressed: bool,
        content_size: Size,
    ) -> Result<&mut ClusterCreator> {
        // Let's get raw cluster
        if let Some(cluster) = self.setup_slot_and_get_to_close(content_size, compressed) {
            self.cluster_writer.write_cluster(cluster, compressed)?;
        }
        Ok(open_cluster_ref!(mut self, compressed).as_mut().unwrap())
    }

    /// Setup a clusterCreator for the slot (compressed or not)
    /// and return a existing clusterCreator to close if a full cluster was present.
    fn setup_slot_and_get_to_close(
        &mut self,
        size: Size,
        compressed: bool,
    ) -> Option<ClusterCreator> {
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

    fn detect_compression(
        &self,
        content: &mut dyn InputReader,
        comp_hint: CompHint,
    ) -> Result<bool> {
        if let Compression::None = self.compression {
            return Ok(false);
        }
        match comp_hint {
            CompHint::Yes => Ok(true),
            CompHint::No => Ok(false),
            CompHint::Detect => {
                let mut head = Vec::with_capacity(4 * 1024);
                {
                    content.take(4 * 1024).read_to_end(&mut head)?;
                }
                let entropy = shannon_entropy(&head)?;
                content.seek(SeekFrom::Start(0))?;
                Ok(entropy <= 6.0)
            }
        }
    }

    pub fn add_content(
        &mut self,
        mut content: Box<dyn InputReader>,
        comp_hint: CompHint,
    ) -> Result<ContentAddress> {
        let content_size = content.size();
        self.progress.content_added(content_size);
        let should_compress = self.detect_compression(content.as_mut(), comp_hint)?;
        let cluster = self.get_open_cluster(should_compress, content_size)?;
        let content_info = cluster.add_content(content)?;
        self.content_infos.push(content_info);
        let content_id = ((self.content_infos.len() - 1) as u32).into();
        Ok(ContentAddress::new(self.pack_id, content_id))
    }
}

impl<O: PackRecipient + 'static + ?Sized> ContentAdder for ContentPackCreator<O> {
    fn add_content(
        &mut self,
        content: Box<dyn InputReader>,
        comp_hint: CompHint,
    ) -> Result<ContentAddress> {
        self.add_content(content, comp_hint)
    }
}

impl<O: PackRecipient + 'static + ?Sized> ContentPackCreator<O> {
    pub fn finalize(mut self) -> Result<(Box<O>, PackData)> {
        info!("======= Finalize creation =======");

        if let Some(cluster) = self.raw_open_cluster.take() {
            if !cluster.is_empty() {
                self.cluster_writer.write_cluster(cluster, false)?;
            }
        }
        if let Some(cluster) = self.comp_open_cluster.take() {
            if !cluster.is_empty() {
                self.cluster_writer.write_cluster(cluster, true)?;
            }
        }

        info!("----- Finalize cluster_writer -----");
        let (mut file, cluster_addresses) = self.cluster_writer.finalize()?;
        let clusters_offset = file.tell();

        let mut buffered = std::io::BufWriter::new(file);

        info!("----- Write cluster addresses -----");
        let nb_clusters = cluster_addresses.len();
        buffered.ser_callable(&|ser| {
            for address in &cluster_addresses {
                address.get().serialize(ser)?;
            }
            Ok(())
        })?;

        info!("----- Write content info -----");
        let content_infos_offset = buffered.tell();

        buffered.ser_callable(&|ser| {
            for content_info in &self.content_infos {
                content_info.serialize(ser)?;
            }
            Ok(())
        })?;
        let check_offset = buffered.tell();
        let pack_size: Size =
            (check_offset + CheckKind::Blake3.block_size() + PackHeader::BLOCK_SIZE).into();
        buffered.rewind()?;

        info!("----- Write pack header -----");
        let pack_header = PackHeader::new(
            PackKind::Content,
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
        );
        buffered.ser_write(&pack_header)?;

        info!("----- Write content pack header -----");
        let header = ContentPackHeader::new(
            self.free_data,
            clusters_offset,
            (nb_clusters as u32).into(),
            content_infos_offset,
            (self.content_infos.len() as u32).into(),
        );
        buffered.ser_write(&header)?;

        buffered.flush()?;
        let mut file = buffered.into_inner().unwrap();

        info!("----- Compute checksum -----");
        file.rewind()?;
        let check_info = CheckInfo::new_blake3(&mut file)?;
        file.ser_write(&check_info)?;

        file.rewind()?;
        let mut tail_buffer = [0u8; PackHeader::BLOCK_SIZE];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        Ok((
            file,
            PackData {
                uuid: pack_header.uuid,
                pack_id: self.pack_id,
                pack_kind: PackKind::Content,
                free_data: Default::default(),
                pack_size,
                check_info,
            },
        ))
    }
}
