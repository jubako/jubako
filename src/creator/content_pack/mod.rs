mod cluster;

use crate::bases::*;
use crate::common::{CompressionType, ContentInfo, ContentPackHeader, PackHeaderInfo};
use crate::creator::{Embedded, PackData};
use cluster::ClusterCreator;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::cell::Cell;

fn shannon_entropy(data: &mut Stream) -> Result<f32> {
    let mut entropy = 0.0;
    let mut counts = [0; 256];
    let size = std::cmp::min(1024, data.size().into_usize());

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

pub struct ContentPackCreator {
    app_vendor_id: u32,
    pack_id: PackId,
    free_data: FreeData40,
    content_infos: Vec<ContentInfo>,
    raw_open_cluster: Option<ClusterCreator>,
    comp_open_cluster: Option<ClusterCreator>,
    next_cluster_id: Cell<usize>,
    cluster_addresses: Vec<SizedOffset>,
    path: PathBuf,
    file: Option<File>,
    compression: CompressionType,
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
    ) -> Self {
        ContentPackCreator {
            app_vendor_id,
            pack_id,
            free_data,
            content_infos: vec![],
            raw_open_cluster: None,
            comp_open_cluster: None,
            next_cluster_id: Cell::new(0),
            cluster_addresses: vec![],
            path: path.as_ref().into(),
            file: None,
            compression,
        }
    }

    fn open_cluster(&self, compressed: bool) -> ClusterCreator {
        let cluster_id = self.next_cluster_id.replace(self.next_cluster_id.get()+1);
        ClusterCreator::new(
            cluster_id,
            if compressed {
                self.compression
            } else {
                CompressionType::None
            },
        )
    }

    fn write_cluster(&mut self, cluster: ClusterCreator) -> Result<()> {
        let data_size = cluster.write_data(self.file.as_mut().unwrap())?;
        let cluster_offset = self.file.as_mut().unwrap().tell();
        cluster.write_tail(self.file.as_mut().unwrap(), data_size)?;
        if self.cluster_addresses.len() <= cluster.index() {
            self.cluster_addresses.resize(
                cluster.index() + 1,
                SizedOffset {
                    size: Size::zero(),
                    offset: Offset::zero(),
                },
            );
        }
        self.cluster_addresses[cluster.index()] = SizedOffset {
            size: cluster.tail_size(),
            offset: cluster_offset,
        };
        Ok(())
    }

    fn get_open_cluster(&mut self, content: &mut Stream) -> Result<&mut ClusterCreator> {
        let entropy = shannon_entropy(content)?;
        content.seek(Offset::zero());
        let compress_content = entropy <= 6.0;
        // Let's get raw cluster
        if let Some(cluster) = self.cluster_to_close(content.size(), compress_content) {
            self.write_cluster(cluster)?;
        }
        Ok(open_cluster_ref!(mut self, compress_content).as_mut().unwrap())
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

    pub fn add_content(&mut self, content: &mut Stream) -> Result<ContentIdx> {
        let cluster = self.get_open_cluster(content)?;
        let content_info = cluster.add_content(content)?;
        self.content_infos.push(content_info);
        Ok(((self.content_infos.len() - 1) as u32).into())
    }

    pub fn finalize(mut self) -> Result<PackData> {
        assert!(self.file.is_some());
        if let Some(cluster) = self.raw_open_cluster.take() {
            if !cluster.is_empty() {
                self.write_cluster(cluster)?;
            }
        }
        if let Some(cluster) = self.comp_open_cluster.take() {
            if !cluster.is_empty() {
                self.write_cluster(cluster)?;
            }
        }
        let file = self.file.as_mut().unwrap();
        let clusters_offset = file.tell();
        for address in &self.cluster_addresses {
            address.write(file)?;
        }
        let content_infos_offset = file.tell();
        for content_info in &self.content_infos {
            content_info.write(file)?;
        }
        let check_offset = file.tell();
        let pack_size: Size = (check_offset + 33 + 64).into();
        file.rewind()?;
        let header = ContentPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
            self.free_data,
            clusters_offset,
            (self.cluster_addresses.len() as u32).into(),
            content_infos_offset,
            (self.content_infos.len() as u32).into(),
        );
        header.write(file)?;
        file.rewind()?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(file, &mut hasher)?;
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
            reader: Reader::new(FileSource::new(self.file.unwrap()), End::None),
            check_info_pos: check_offset,
            embedded: Embedded::No(self.path),
        })
    }
}
