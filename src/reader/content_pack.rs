mod cluster;

use crate::bases::*;
use crate::common::{CheckInfo, ContentPackHeader, EntryInfo, Pack, PackKind};
use cluster::Cluster;
use std::cell::Cell;
use std::io::Read;
use uuid::Uuid;

pub struct ContentPack {
    header: ContentPackHeader,
    entry_infos: ArrayReader<EntryInfo, u32>,
    cluster_ptrs: ArrayReader<SizedOffset, u32>,
    reader: Box<dyn Reader>,
    check_info: Cell<Option<CheckInfo>>,
}

impl ContentPack {
    pub fn new(reader: Box<dyn Reader>) -> Result<Self> {
        let header = ContentPackHeader::produce(reader.create_stream_all().as_mut())?;
        let entry_infos =
            ArrayReader::new_from_reader(reader.as_ref(), header.entry_ptr_pos, header.entry_count);
        let cluster_ptrs = ArrayReader::new_from_reader(
            reader.as_ref(),
            header.cluster_ptr_pos,
            header.cluster_count,
        );
        Ok(ContentPack {
            header,
            entry_infos,
            cluster_ptrs,
            reader,
            check_info: Cell::new(None),
        })
    }

    pub fn get_entry_count(&self) -> Count<u32> {
        self.header.entry_count
    }

    pub fn get_content(&self, index: Idx<u32>) -> Result<Box<dyn Reader>> {
        if !index.is_valid(self.header.entry_count) {
            return Err(Error::Arg);
        }
        let entry_info = self.entry_infos.index(index);
        if !entry_info.cluster_index.is_valid(self.header.cluster_count) {
            return Err(format_error!(&format!(
                "Cluster index ({}) is not valid in regard of cluster count ({})",
                entry_info.cluster_index, self.header.cluster_count
            )));
        }
        let cluster_info = self.cluster_ptrs.index(entry_info.cluster_index);
        let cluster = Cluster::new(self.reader.as_ref(), cluster_info)?;
        cluster.get_reader(entry_info.blob_index)
    }

    pub fn get_free_data(&self) -> &[u8] {
        self.header.free_data.as_ref()
    }
}

impl Pack for ContentPack {
    fn kind(&self) -> PackKind {
        self.header.pack_header.magic
    }
    fn app_vendor_id(&self) -> u32 {
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
            let mut checkinfo_stream = self
                .reader
                .create_stream_from(self.header.pack_header.check_info_pos);
            let check_info = CheckInfo::produce(checkinfo_stream.as_mut())?;
            self.check_info.set(Some(check_info));
        }
        let mut check_stream = self
            .reader
            .create_stream_to(End::Offset(self.header.pack_header.check_info_pos));
        self.check_info
            .get()
            .unwrap()
            .check(&mut check_stream.as_mut() as &mut dyn Read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contentpack() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x63, // magic off:0
            0x01, 0x00, 0x00, 0x00, // app_vendor_id off:4
            0x01, // major_version off:8
            0x02, // minor_version off:9
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid off:10
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding off:26
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xD3, // file_size off:32
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAB, // check_info_pos off:40
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved off:48
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x80, // entry_ptr_pos off:64
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8C, // cluster_ptr_pos off:72
            0x00, 0x00, 0x00, 0x03, // entry count off:80
            0x00, 0x00, 0x00, 0x01, // cluster count off:84
        ];
        content.extend_from_slice(&[0xff; 40]); // free_data off:88
        content.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00, // first entry info off:128
            0x00, 0x00, 0x00, 0x01, // second entry info off: 132
            0x00, 0x00, 0x00, 0x02, // third entry info off: 136
            0x00, 0x08, // first (and only) cluster size off:140
            0x00, 0x00, 0x00, 0x00, 0x00, 0xA3, // first (and only) ptr pos. off:143
            // Cluster off:148
            0x11, 0x12, 0x13, 0x14, 0x15, // Data of blob 0
            0x21, 0x22, 0x23, // Data of blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of blob 2
            0x00, // compression off: 148+15 = 163
            0x01, // offset_size
            0x00, 0x03, // blob_count
            0x0f, // raw data size
            0x0f, // Data size
            0x05, // Offset of blob 1
            0x08, // Offset of blob 2
        ]); // end 163+8 = 171
        let hash = blake3::hash(&content);
        content.push(0x01); // check info off: 171
        content.extend(hash.as_bytes()); // end : 171+32 = 203
        let reader = Box::new(BufReader::new(content, End::None));
        let content_pack = ContentPack::new(reader).unwrap();
        assert_eq!(content_pack.get_entry_count(), Count(3));
        assert_eq!(content_pack.app_vendor_id(), 0x01000000_u32);
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
            let sub_reader = content_pack.get_content(Idx(0_u32)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream_all();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let sub_reader = content_pack.get_content(Idx(1_u32)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream_all();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let sub_reader = content_pack.get_content(Idx(2_u32)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream_all();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }
}
