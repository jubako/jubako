mod cluster;

use crate::bases::*;
use crate::common::{CheckInfo, ContentInfo, ContentPackHeader, Pack, PackHeader, PackKind};
use cluster::Cluster;
use fxhash::FxBuildHasher;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;

use super::ByteRegion;

pub struct ContentPack {
    pack_header: PackHeader,
    header: ContentPackHeader,
    content_infos: ArrayReader<ContentInfo, u32>,
    cluster_ptrs: ArrayReader<SizedOffset, u32>,
    cluster_cache: Mutex<LruCache<ClusterIdx, Arc<Cluster>, FxBuildHasher>>,
    reader: Reader,
    check_info: OnceLock<CheckInfo>,
}

impl ContentPack {
    pub fn new(reader: Reader) -> Result<Self> {
        let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero())?;
        if pack_header.magic != PackKind::Content {
            return Err(format_error!("Pack Magic is not ContentPack"));
        }

        let header =
            reader.parse_block_at::<ContentPackHeader>(Offset::from(PackHeader::BLOCK_SIZE))?;
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
            pack_header,
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

    pub fn get_content(&self, index: ContentIdx) -> Result<Option<ByteRegion>> {
        if !index.is_valid(*self.header.content_count) {
            return Ok(None);
        }
        let content_info = self.content_infos.index(*index)?;
        if !content_info
            .cluster_index
            .is_valid(*self.header.cluster_count)
        {
            return Err(format_error!(&format!(
                "Cluster index ({}) is not valid in regard of cluster count ({})",
                content_info.cluster_index, self.header.cluster_count
            )));
        }
        let cluster = self.get_cluster(content_info.cluster_index)?;
        Ok(Some(cluster.get_bytes(content_info.blob_index)?))
    }

    pub fn get_free_data(&self) -> &[u8] {
        self.header.free_data.as_ref()
    }
}

#[cfg(feature = "explorable_serde")]
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
impl graphex::Node for ContentPack {
    fn next(&self, key: &str) -> graphex::ExploreResult {
        if let Some(item) = key.strip_prefix("e.") {
            let index = item
                .parse::<u32>()
                .map_err(|e| graphex::Error::key(&format!("{e}")))?;
            let index = ContentIdx::from(index);
            let content_info = self.content_infos.index(*index)?;
            Ok(Box::new(content_info).into())
        } else if let Some(item) = key.strip_prefix("c.") {
            let index = item
                .parse::<u32>()
                .map_err(|e| graphex::Error::key(&format!("{e}")))?;
            let cluster_info = self.cluster_ptrs.index(index.into())?;
            Ok(Box::new(self.reader.parse_data_block::<Cluster>(cluster_info)?).into())
        } else {
            Err(graphex::Error::key(key))
        }
    }

    fn display(&self) -> &dyn graphex::Display {
        self
    }

    #[cfg(feature = "explorable_serde")]
    fn serde(&self) -> Option<&dyn erased_serde::Serialize> {
        Some(self)
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for ContentPack {
    fn header_footer(&self) -> Option<(String, String)> {
        Some(("ContentPack(".to_string(), ")".to_string()))
    }

    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        use yansi::Paint;
        out.field("uuid", &self.uuid().to_string())?;
        out.field(
            &format!("entries count ({})", "e.<N>".bold()),
            &self.header.content_count.into_u64(),
        )?;
        out.field(
            &format!("clusters count ({})", "c.<N>".bold()),
            &self.header.cluster_count.into_u64(),
        )?;
        out.field("freeData", &self.header.free_data)
    }
}

impl Pack for ContentPack {
    fn kind(&self) -> PackKind {
        self.pack_header.magic
    }
    fn app_vendor_id(&self) -> VendorId {
        self.pack_header.app_vendor_id
    }
    fn version(&self) -> (u8, u8) {
        (
            self.pack_header.major_version,
            self.pack_header.minor_version,
        )
    }
    fn uuid(&self) -> Uuid {
        self.pack_header.uuid
    }
    fn size(&self) -> Size {
        self.pack_header.file_size
    }
    fn check(&self) -> Result<bool> {
        if self.check_info.get().is_none() {
            let _ = self.check_info.set(self.reader.parse_block_in::<CheckInfo>(
                self.pack_header.check_info_pos,
                self.pack_header.check_info_size(),
            )?);
        }
        let check_info = self.check_info.get().unwrap();
        let mut check_stream = self.reader.create_stream(
            Offset::zero(),
            Size::from(self.pack_header.check_info_pos),
            false,
        )?;
        Ok(check_info.check(&mut check_stream)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_contentpack() -> Result<()> {
        let mut content = vec![];

        // Pack header offset 0/0x00
        content.extend_from_slice(&[
            0x6a, 0x62, 0x6b, 0x63, // magic
            0x00, 0x00, 0x00, 0x01, // app_vendor_id
            0x00, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0x1C, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // file_size
            0xB7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, // reserved
        ]);
        content.extend_from_slice(&[0xE8, 0x9E, 0x15, 0x60]); // CRC

        // ContentPack header offset 64/0x40
        content.extend_from_slice(&[
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // entry_ptr_pos
            0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // cluster_ptr_pos
            0x03, 0x00, 0x00, 0x00, // entry count
            0x01, 0x00, 0x00, 0x00, // cluster count
        ]);
        content.extend_from_slice(&[0xff; 36]); // free_data
        content.extend_from_slice(&[0x93, 0xF9, 0x45, 0x68]); // CRC

        // Entry ptr array offset 128/0x80 (entry_ptr_pos)
        content.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00, // first entry info
            0x01, 0x00, 0x00, 0x00, // second entry info
            0x02, 0x00, 0x00, 0x00, // third entry info
        ]);
        content.extend_from_slice(&[0x84, 0xC1, 0x1C, 0xD2]); // CRC

        // Cluster ptr array offset 128 + 16 = 144/0x90 (cluste_ptr_pos)
        content.extend_from_slice(&[
            0x08, 0x00, // first (and only) cluster size
            0xAB, 0x00, 0x00, 0x00, 0x00, 0x00, // first (and only) ptr pos.
        ]);
        content.extend_from_slice(&[0x35, 0x23, 0x26, 0x1E]); // CRC

        // Cluster data offset 144 + 12 = 156/0x9C
        content.extend_from_slice(&[
            0x11, 0x12, 0x13, 0x14, 0x15, // Data of blob 0
            0x21, 0x22, 0x23, // Data of blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of blob 2
        ]);

        // Cluster tail offset 156 + 15(0x0f) = 171/0xAB (cluster_header)
        content.extend_from_slice(&[
            0x00, // compression
            0x01, // offset_size
            0x03, 0x00, // blob_count
            0x0f, // raw data size
            0x0f, // Data size
            0x05, // Offset of blob 1
            0x08, // Offset of blob 2
        ]);
        content.extend_from_slice(&[0x42, 0xCC, 0x02, 0x58]); // CRC

        // Check info offset 171 + 8 + 4 = 183/0xB7 (check_info_pos)
        let hash = blake3::hash(&content);
        content.push(0x01);
        content.extend(hash.as_bytes());
        content.extend_from_slice(&[0x78, 0x20, 0x61, 0xB7]); // CRC

        // Footer offset 183 + 33 + 4 = 220/0xDC
        let mut footer = [0; 64];
        footer.copy_from_slice(&content[..64]);
        footer.reverse();
        content.extend_from_slice(&footer);

        // FileSize 220 + 64 = 284/0x011C (file_size)

        let content_pack = ContentPack::new(content.into())?;
        assert_eq!(content_pack.get_content_count(), ContentCount::from(3));
        assert_eq!(content_pack.app_vendor_id(), VendorId::from([0, 0, 0, 1]));
        assert_eq!(content_pack.version(), (0, 2));
        assert_eq!(
            content_pack.uuid(),
            Uuid::from_bytes([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f
            ])
        );
        assert_eq!(content_pack.get_free_data(), [0xff; 24]);
        assert!(&content_pack.check()?);

        {
            let bytes = content_pack
                .get_content(ContentIdx::from(0))?
                .expect("0 is a valid content idx");
            assert_eq!(bytes.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = bytes.stream();
            stream.read_to_end(&mut v)?;
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let bytes = content_pack
                .get_content(ContentIdx::from(1))?
                .expect("1 is a valid content idx");
            assert_eq!(bytes.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = bytes.stream();
            stream.read_to_end(&mut v)?;
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let bytes = content_pack
                .get_content(ContentIdx::from(2))?
                .expect("2 is a valid content idx");
            assert_eq!(bytes.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = bytes.stream();
            stream.read_to_end(&mut v)?;
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
        Ok(())
    }
}
