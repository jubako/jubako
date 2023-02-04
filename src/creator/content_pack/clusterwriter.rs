use super::cluster::ClusterCreator;
use crate::bases::*;
use crate::common::CompressionType;
use crate::creator::private::WritableTell;
use std::fs::File;

pub struct ClusterWriter {
    cluster_addresses: Vec<SizedOffset>,
    pub compression: CompressionType,
    file: File,
}

impl ClusterWriter {
    pub fn new(file: File, compression: CompressionType) -> Self {
        Self {
            cluster_addresses: vec![],
            compression,
            file,
        }
    }

    pub fn write_cluster(&mut self, mut cluster: ClusterCreator) -> Result<()> {
        let sized_offset = cluster.write(&mut self.file)?;
        if self.cluster_addresses.len() <= cluster.index() {
            self.cluster_addresses.resize(
                cluster.index() + 1,
                SizedOffset {
                    size: Size::zero(),
                    offset: Offset::zero(),
                },
            );
        }
        self.cluster_addresses[cluster.index()] = sized_offset;
        Ok(())
    }

    pub fn finalize(self) -> Result<(File, Vec<SizedOffset>)> {
        Ok((self.file, self.cluster_addresses))
    }
}
