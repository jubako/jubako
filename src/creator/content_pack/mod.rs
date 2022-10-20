mod cluster;

use super::{CheckInfo, PackInfo};
use crate::bases::*;
use crate::common::{CompressionType, ContentPackHeader, EntryInfo, PackHeaderInfo, PackPos};
use cluster::ClusterCreator;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use typenum::U40;

pub struct ContentPackCreator {
    app_vendor_id: u32,
    pack_id: Id<u8>,
    free_data: FreeData<U40>,
    blob_addresses: Vec<EntryInfo>,
    open_cluster: Option<ClusterCreator>,
    cluster_addresses: Vec<SizedOffset>,
    path: PathBuf,
    file: Option<File>,
    compression: CompressionType,
}

impl ContentPackCreator {
    pub fn new<P: AsRef<Path>>(
        path: P,
        pack_id: Id<u8>,
        app_vendor_id: u32,
        free_data: FreeData<U40>,
        compression: CompressionType,
    ) -> Self {
        ContentPackCreator {
            app_vendor_id,
            pack_id,
            free_data,
            blob_addresses: vec![],
            open_cluster: None,
            cluster_addresses: vec![],
            path: path.as_ref().into(),
            file: None,
            compression,
        }
    }

    fn open_cluster(&mut self) {
        assert!(self.open_cluster.is_none());
        let cluster_id = self.cluster_addresses.len();
        self.open_cluster = Some(ClusterCreator::new(cluster_id, self.compression));
    }

    fn write_cluster(&mut self) -> Result<()> {
        let cluster = self.open_cluster.as_ref().unwrap();
        let data_size = cluster.write_data(self.file.as_mut().unwrap())?;
        let cluster_offset = self.file.as_mut().unwrap().tell();
        cluster.write_tail(self.file.as_mut().unwrap(), data_size)?;
        if self.cluster_addresses.len() <= cluster.index() {
            self.cluster_addresses.resize(
                cluster.index() + 1,
                SizedOffset {
                    size: Size(0),
                    offset: Offset(0),
                },
            );
        }
        self.cluster_addresses[cluster.index()] = SizedOffset {
            size: cluster.tail_size(),
            offset: cluster_offset,
        };
        self.open_cluster = None;
        Ok(())
    }

    fn get_open_cluster(&mut self, size: Size) -> Result<&mut ClusterCreator> {
        if let Some(cluster) = self.open_cluster.as_ref() {
            if cluster.is_full(size) {
                self.write_cluster()?;
            }
        }
        if self.open_cluster.is_none() {
            self.open_cluster();
        }
        Ok(self.open_cluster.as_mut().unwrap())
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

    pub fn add_content(&mut self, content: &mut dyn Stream) -> Result<Idx<u32>> {
        let cluster = self.get_open_cluster(content.size())?;
        let entry_info = cluster.add_content(content)?;
        self.blob_addresses.push(entry_info);
        Ok(((self.blob_addresses.len() - 1) as u32).into())
    }

    pub fn finalize(&mut self) -> Result<PackInfo> {
        assert!(self.file.is_some());
        if let Some(cluster) = self.open_cluster.as_ref() {
            if !cluster.is_empty() {
                self.write_cluster()?;
            }
        }
        let file = self.file.as_mut().unwrap();
        let clusters_offset = file.tell();
        for address in &self.cluster_addresses {
            address.write(file)?;
        }
        let entries_offset = file.tell();
        for address in &self.blob_addresses {
            address.write(file)?;
        }
        let check_offset = file.tell();
        let pack_size: Size = (check_offset + 33).into();
        file.rewind()?;
        let header = ContentPackHeader::new(
            PackHeaderInfo::new(self.app_vendor_id, pack_size, check_offset),
            self.free_data,
            clusters_offset,
            (self.cluster_addresses.len() as u32).into(),
            entries_offset,
            (self.blob_addresses.len() as u32).into(),
        );
        header.write(file)?;
        file.rewind()?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_u8(1)?;
        file.write_all(hash.as_bytes())?;
        Ok(PackInfo {
            uuid: header.pack_header.uuid,
            pack_id: self.pack_id,
            free_data: FreeData::clone_from_slice(&[0; 103]),
            pack_size: pack_size.0,
            check_info: CheckInfo::new_blake3(hash.as_bytes()),
            pack_pos: PackPos::Path(self.path.to_str().unwrap().into()),
        })
    }
}
