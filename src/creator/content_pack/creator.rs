use super::cluster::ClusterCreator;
use super::clusterwriter::ClusterWriterProxy;
use super::{ContentAdder, Progress};
use crate::bases::*;
use crate::common::{
    CheckInfo, ContentAddress, ContentInfo, ContentPackHeader, PackHeaderInfo, PackKind,
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

pub struct ContentPackCreator<O: PackRecipient> {
    app_vendor_id: VendorId,
    pack_id: PackId,
    free_data: ContentPackFreeData,
    content_infos: Vec<ContentInfo>,
    raw_open_cluster: Option<ClusterCreator>,
    comp_open_cluster: Option<ClusterCreator>,
    next_cluster_id: Cell<u32>,
    cluster_writer: ClusterWriterProxy<O>,
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
        free_data: ContentPackFreeData,
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
        free_data: ContentPackFreeData,
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

impl<O: PackRecipient + 'static> ContentPackCreator<O> {
    pub fn new_from_output(
        file: O,
        pack_id: PackId,
        app_vendor_id: VendorId,
        free_data: ContentPackFreeData,
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
        mut file: O,
        pack_id: PackId,
        app_vendor_id: VendorId,
        free_data: ContentPackFreeData,
        compression: Compression,
        progress: Arc<dyn Progress>,
    ) -> Result<Self> {
        file.seek(SeekFrom::Start(ContentPackHeader::SIZE as u64))?;
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

    fn detect_compression<R: InputReader + 'static>(&self, content: &mut R) -> Result<bool> {
        if let Compression::None = self.compression {
            return Ok(false);
        }
        let mut head = Vec::with_capacity(4 * 1024);
        {
            content.by_ref().take(4 * 1024).read_to_end(&mut head)?;
        }
        let entropy = shannon_entropy(&head)?;
        content.seek(SeekFrom::Start(0))?;
        Ok(entropy <= 6.0)
    }
}

impl<O: PackRecipient + 'static> ContentAdder for ContentPackCreator<O> {
    fn add_content<R: InputReader + 'static>(&mut self, mut content: R) -> Result<ContentAddress> {
        let content_size = content.size();
        self.progress.content_added(content_size);
        let should_compress = self.detect_compression(&mut content)?;
        let cluster = self.get_open_cluster(should_compress, content_size)?;
        let content_info = cluster.add_content(content)?;
        self.content_infos.push(content_info);
        let content_id = ((self.content_infos.len() - 1) as u32).into();
        Ok(ContentAddress::new(self.pack_id, content_id))
    }
}

impl<O: PackRecipient + 'static> ContentPackCreator<O> {
    pub fn finalize(mut self) -> Result<(O, PackData)> {
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

        info!("----- Write cluster addresses -----");
        let nb_clusters = cluster_addresses.len();

        let mut buffered = std::io::BufWriter::new(file);

        for address in cluster_addresses {
            address.get().write(&mut buffered)?;
        }

        info!("----- Write content info -----");
        let content_infos_offset = buffered.tell();
        for content_info in &self.content_infos {
            content_info.write(&mut buffered)?;
        }
        let check_offset = buffered.tell();
        let pack_size: Size = (check_offset + 33 + 64).into();
        buffered.rewind()?;

        info!("----- Write header -----");
        let header = ContentPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
            self.free_data,
            clusters_offset,
            (nb_clusters as u32).into(),
            content_infos_offset,
            (self.content_infos.len() as u32).into(),
        );
        header.write(&mut buffered)?;

        buffered.flush()?;
        let mut file = buffered.into_inner().unwrap();

        info!("----- Compute checksum -----");
        file.rewind()?;
        let check_info = CheckInfo::new_blake3(&mut file)?;
        check_info.write(&mut file)?;

        file.rewind()?;
        let mut tail_buffer = [0u8; 64];
        file.read_exact(&mut tail_buffer)?;
        tail_buffer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&tail_buffer)?;

        Ok((
            file,
            PackData {
                uuid: header.pack_header.uuid,
                pack_id: self.pack_id,
                pack_kind: PackKind::Content,
                free_data: Default::default(),
                pack_size,
                check_info,
            },
        ))
    }
}
