mod cluster;

use crate::bases::*;
use crate::common::{CheckInfo, ContentInfo, ContentPackHeader, Pack, PackKind};
use cluster::Cluster;
use fxhash::FxBuildHasher;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;

use super::ByteRegion;

pub struct ContentPack {
    header: ContentPackHeader,
    content_infos: ArrayReader<ContentInfo, u32>,
    cluster_ptrs: ArrayReader<SizedOffset, u32>,
    cluster_cache: Mutex<LruCache<ClusterIdx, Arc<Cluster>, FxBuildHasher>>,
    reader: Reader,
    check_info: OnceLock<CheckInfo>,
}

impl ContentPack {
    pub fn new(reader: Reader) -> Result<Self> {
        let header = reader.parse_block_at::<ContentPackHeader>(Offset::zero())?;
        let content_infos = ArrayReader::new_memory_from_reader(
            &reader,
            header.content_ptr_pos,
            *header.content_count,
        )?;
        let cluster_ptrs = ArrayReader::new_memory_from_reader(
            &reader,
            header.cluster_ptr_pos,
            *header.cluster_count,
        )?;
        Ok(ContentPack {
            header,
            content_infos,
            cluster_ptrs,
            cluster_cache: Mutex::new(LruCache::with_hasher(
                NonZeroUsize::new(40).unwrap(),
                FxBuildHasher::default(),
            )),
            reader,
            check_info: OnceLock::new(),
        })
    }

    pub fn get_content_count(&self) -> ContentCount {
        self.header.content_count
    }

    fn _get_cluster(&self, cluster_index: ClusterIdx) -> Result<Arc<Cluster>> {
        let cluster_info = self.cluster_ptrs.index(*cluster_index)?;
        let cluster = self.reader.parse_data_block::<Cluster>(cluster_info)?;
        Ok(Arc::new(cluster))
    }

    fn get_cluster(&self, cluster_index: ClusterIdx) -> Result<Arc<Cluster>> {
        let mut cache = self.cluster_cache.lock().unwrap();
        let cached = cache.try_get_or_insert(cluster_index, || self._get_cluster(cluster_index))?;
        Ok(cached.clone())
    }

    pub fn get_content(&self, index: ContentIdx) -> Result<ByteRegion> {
        if !index.is_valid(self.header.content_count) {
            return Err(Error::new_arg());
        }
        let content_info = self.content_infos.index(*index)?;
        if !content_info
            .cluster_index
            .is_valid(self.header.cluster_count)
        {
            return Err(format_error!(&format!(
                "Cluster index ({}) is not valid in regard of cluster count ({})",
                content_info.cluster_index, self.header.cluster_count
            )));
        }
        let cluster = self.get_cluster(content_info.cluster_index)?;
        cluster.get_bytes(content_info.blob_index)
    }

    pub fn get_free_data(&self) -> &[u8] {
        self.header.free_data.as_ref()
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for ContentPack {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut cont = serializer.serialize_struct("ContentPack", 4)?;
        cont.serialize_field("uuid", &self.uuid())?;
        cont.serialize_field("#entries", &self.header.content_count)?;
        cont.serialize_field("#clusters", &self.header.cluster_count)?;
        cont.serialize_field("freeData", &self.header.free_data)?;
        cont.end()
    }
}

#[cfg(feature = "explorable")]
impl Explorable for ContentPack {
    fn explore_one(&self, item: &str) -> Result<Option<Box<dyn Explorable>>> {
        if let Some(item) = item.strip_prefix("e.") {
            let index = item
                .parse::<u32>()
                .map_err(|e| Error::from(format!("{e}")))?;
            let index = ContentIdx::from(index);
            let content_info = self.content_infos.index(*index)?;
            Ok(Some(Box::new(content_info)))
        } else if let Some(item) = item.strip_prefix("c.") {
            let index = item
                .parse::<u32>()
                .map_err(|e| Error::from(format!("{e}")))?;
            let cluster_info = self.cluster_ptrs.index(index.into())?;
            Ok(Some(Box::new(
                self.reader.parse_data_block::<Cluster>(cluster_info)?,
            )))
        } else {
            Ok(None)
        }
    }
}
impl Pack for ContentPack {
    fn kind(&self) -> PackKind {
        self.header.pack_header.magic
    }
    fn app_vendor_id(&self) -> VendorId {
        self.header.pack_header.app_vendor_id
    }
    fn version(&self) -> (u8, u8) {
        (
            self.header.pack_header.major_version,
            self.header.pack_header.minor_version,
        )
    }
    fn uuid(&self) -> Uuid {
        self.header.pack_header.uuid
    }
    fn size(&self) -> Size {
        self.header.pack_header.file_size
    }
    fn check(&self) -> Result<bool> {
        if self.check_info.get().is_none() {
            let _ = self.check_info.set(self.reader.parse_block_in::<CheckInfo>(
                self.header.pack_header.check_info_pos,
                self.header.pack_header.check_info_size(),
            )?);
        }
        let check_info = self.check_info.get().unwrap();
        let mut check_stream = self.reader.create_stream(
            Offset::zero(),
            Size::from(self.header.pack_header.check_info_pos),
        );
        check_info.check(&mut check_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_contentpack() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x63, // magic off:0
            0x00, 0x00, 0x00, 0x01, // app_vendor_id off:4
            0x01, // major_version off:8
            0x02, // minor_version off:9
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid off:10
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding off:26
            0x0C, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // file_size off:32
            0xAB, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // check_info_pos off:40
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved off:48
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // entry_ptr_pos off:64
            0x8C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // cluster_ptr_pos off:72
            0x03, 0x00, 0x00, 0x00, // entry count off:80
            0x01, 0x00, 0x00, 0x00, // cluster count off:84
        ];
        content.extend_from_slice(&[0xff; 40]); // free_data off:88

        // Offset 128/0x80 (entry_ptr_pos)
        content.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00, // first entry info off:128
            0x01, 0x00, 0x00, 0x00, // second entry info off: 132
            0x02, 0x00, 0x00, 0x00, // third entry info off: 136
        ]);

        // Offset 128 + 12 = 140/0x8C (cluste_ptr_pos)
        content.extend_from_slice(&[
            0x08, 0x00, // first (and only) cluster size off:140
            0xA3, 0x00, 0x00, 0x00, 0x00, 0x00, // first (and only) ptr pos. off:143
        ]);

        // Offset 140 + 8 = 148/0x94 (cluster_data)
        content.extend_from_slice(&[
            // Cluster off:148
            0x11, 0x12, 0x13, 0x14, 0x15, // Data of blob 0
            0x21, 0x22, 0x23, // Data of blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of blob 2
        ]);

        // Offset 148 + 15(0x0f) = 163/0xA3 (cluster_header)
        content.extend_from_slice(&[
            0x00, // compression off: 148+15 = 163
            0x01, // offset_size
            0x03, 0x00, // blob_count
            0x0f, // raw data size
            0x0f, // Data size
            0x05, // Offset of blob 1
            0x08, // Offset of blob 2
        ]);

        // Offset end 163 + 8 = 171/0xAB (check_info_pos)
        let hash = blake3::hash(&content);
        content.push(0x01); // check info off: 171
        content.extend(hash.as_bytes()); // end : 171+32 = 203

        // Offset 171 + 33 = 204/0xCC

        // Add footer
        let mut footer = [0; 64];
        footer.copy_from_slice(&content[..64]);
        footer.reverse();
        content.extend_from_slice(&footer);

        // FileSize 204 + 64 = 268/0x010C (file_size)

        let content_pack = ContentPack::new(content.into()).unwrap();
        assert_eq!(content_pack.get_content_count(), ContentCount::from(3));
        assert_eq!(
            content_pack.app_vendor_id(),
            VendorId::from([00, 00, 00, 01])
        );
        assert_eq!(content_pack.version(), (1, 2));
        assert_eq!(
            content_pack.uuid(),
            Uuid::from_bytes([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f
            ])
        );
        assert_eq!(&content_pack.get_free_data()[..], &[0xff; 40][..]);
        assert!(&content_pack.check().unwrap());

        {
            let bytes = content_pack.get_content(ContentIdx::from(0)).unwrap();
            assert_eq!(bytes.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = bytes.stream();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let bytes = content_pack.get_content(ContentIdx::from(1)).unwrap();
            assert_eq!(bytes.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = bytes.stream();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let bytes = content_pack.get_content(ContentIdx::from(2)).unwrap();
            assert_eq!(bytes.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = bytes.stream();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }
}
