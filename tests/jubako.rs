use galvanic_test::test_suite;

use std::path::PathBuf;

struct Entry {
    path: String,
    content: String,
    word_count: u16,
}

struct Cluster {
    data: Vec<Vec<u8>>,
    entries_offset: Vec<usize>,
    compression: jubako::CompressionType,
    data_size: Option<usize>,
    tail_size: Option<u16>,
}

impl Cluster {
    pub fn new(compression: jubako::CompressionType, entries: &Vec<Entry>) -> Self {
        let mut data: Vec<Vec<u8>> = vec![];
        let mut entries_offset = vec![];
        let mut current_offset = 0;
        for entry in entries {
            data.push(entry.content.as_bytes().to_vec());
            current_offset += entry.content.as_bytes().len();
            entries_offset.push(current_offset);
        }
        Cluster {
            data,
            entries_offset,
            compression,
            data_size: None,
            tail_size: None,
        }
    }

    pub fn data_bytes(&mut self) -> Vec<u8> {
        match self.compression {
            jubako::CompressionType::None => {
                let mut data = vec![];
                for content in &self.data {
                    data.extend(content);
                }
                self.data_size = Some(data.len());
                data
            }
            _ => unreachable!(),
        }
    }

    pub fn tail_bytes(&mut self) -> Vec<u8> {
        let mut data = vec![];
        data.push(self.compression as u8);
        data.push(0x08); // offset size [TODO] Use better size
        data.extend((self.entries_offset.len() as u16).to_le_bytes()); // blob_count
        data.extend((self.entries_offset[self.entries_offset.len() - 1] as u64).to_le_bytes()); //raw data size
        data.extend((self.data_size.unwrap() as u64).to_le_bytes()); // data size
        for offset in &self.entries_offset[..(self.entries_offset.len() - 1)] {
            data.extend((*offset as u64).to_le_bytes());
        }
        self.tail_size = Some(data.len() as u16);
        data.extend_from_slice(&[0; 4]); // Dummy Crc32
        data
    }

    pub fn tail_size(&self) -> u16 {
        self.tail_size.unwrap()
    }
}

struct KeyStore {
    data: Vec<Vec<u8>>,
    entries_offset: Vec<usize>,
    tail_size: Option<u16>,
}

impl KeyStore {
    pub fn new(entries: &Vec<Entry>) -> Self {
        let mut data: Vec<Vec<u8>> = vec![];
        let mut entries_offset = vec![];
        let mut current_offset = 0;
        for entry in entries {
            data.push(entry.path.as_bytes().to_vec());
            current_offset += entry.path.as_bytes().len();
            entries_offset.push(current_offset);
        }
        KeyStore {
            data,
            entries_offset,
            tail_size: None,
        }
    }

    pub fn data_bytes(&self) -> Vec<u8> {
        let mut data = vec![];
        for content in &self.data {
            data.extend(content);
        }
        data.extend_from_slice(&[0; 4]); // Dummy Crc32
        data
    }

    pub fn tail_bytes(&mut self) -> Vec<u8> {
        let mut data = vec![];
        data.push(0x01); // kind
        data.extend((self.entries_offset.len() as u64).to_le_bytes()); // key count
        data.push(0x08); // offset size [TODO] Use a better size
        data.extend((self.entries_offset[self.entries_offset.len() - 1] as u64).to_le_bytes()); //data size
        for offset in &self.entries_offset[..(self.entries_offset.len() - 1)] {
            data.extend((*offset as u64).to_le_bytes());
        }
        self.tail_size = Some(data.len() as u16);
        data.extend_from_slice(&[0; 4]); // Dummy Crc32
        data
    }

    pub fn tail_size(&self) -> u16 {
        self.tail_size.unwrap()
    }
}

struct IndexStore {
    data: Vec<u8>,
    tail_size: Option<u16>,
}

impl IndexStore {
    pub fn new(entries: &Vec<Entry>) -> Self {
        let mut data: Vec<u8> = vec![];
        let mut idx: u8 = 0;
        for entry in entries {
            // We are creating entry data.
            // Each entry has 3 keys :
            // - The path : A char1[0] + deported(1)
            // - The content : a content address(1)
            // - The words counts : a u16
            data.extend(&[entry.path.as_bytes().len() as u8, idx].to_vec());
            data.extend(&(((idx as u16) << 8) + 0x01_u16).to_le_bytes().to_vec());
            data.extend(&entry.word_count.to_le_bytes().to_vec());
            idx += 1;
        }
        data.extend_from_slice(&[0; 4]); // Dummy Crc32
        IndexStore {
            data,
            tail_size: None,
        }
    }

    pub fn data_bytes(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn tail_bytes(&mut self) -> Vec<u8> {
        let mut data = vec![];
        data.push(0x00); // kind
        data.extend(2_u32.to_le_bytes()); // entry_count
        data.push(0x00); // flags
        data.extend(6_u16.to_le_bytes()); // entry_size
        data.push(0x00); // variant count
        data.push(0x03); // key count
        data.extend(&[0b0101_0001, 0b001_00000, 0x00]); // The first key, a char1[0] + deported(1) idx 0
        data.extend(&[2, b'V', b'0']); // The name of the first key "V0"
        data.extend(&[0b0001_0000]); // The second key, the content address (1 for the pack_id + 1 for the value_id)
        data.extend(&[2, b'V', b'1']); // The name of the second key "V1"
        data.extend(&[0b0010_0001]); // The third key, the u16
        data.extend(&[2, b'V', b'2']); // The name of the third key "V2"
        data.extend((self.data.len() as u64 - 4).to_le_bytes()); //data size
        self.tail_size = Some(data.len() as u16);
        data.extend_from_slice(&[0; 4]); // Dummy Crc32
        data
    }

    pub fn tail_size(&self) -> u16 {
        self.tail_size.unwrap()
    }
}

struct Index {
    store_id: u32,
    entry_count: u32,
    index_key: u8,
    index_name: String,
    tail_size: Option<u16>,
}

impl Index {
    pub fn new(entries: &Vec<Entry>) -> Self {
        Index {
            store_id: 0,
            entry_count: entries.len() as u32,
            index_key: 0,
            index_name: "Super index".to_string(),
            tail_size: None,
        }
    }

    pub fn bytes(&mut self) -> Vec<u8> {
        let mut data = vec![];
        data.extend(self.store_id.to_le_bytes()); // store_id
        data.extend(self.entry_count.to_le_bytes()); // entry_count
        data.extend(0_u32.to_le_bytes()); // entry_offset
        data.extend([0; 4]); // free_data
        data.extend(self.index_key.to_le_bytes()); // index_key
        data.push(self.index_name.len() as u8);
        data.extend(self.index_name.bytes()); // The index name
        self.tail_size = Some(data.len() as u16);
        data.extend_from_slice(&[0; 4]); // Dummy Crc32
        data
    }

    pub fn tail_size(&self) -> u16 {
        self.tail_size.unwrap()
    }
}

struct CheckInfo {
    kind: u8,
    data: Vec<u8>,
}

impl CheckInfo {
    pub fn bytes(&self) -> Vec<u8> {
        let mut data = vec![];
        data.push(self.kind);
        data.extend(&self.data);
        data.extend_from_slice(&[0; 4]); // Dummy CRC32
        data
    }

    pub fn size(&self) -> u64 {
        (self.data.len() as u64) + 1 + 4
    }
}

struct PackInfo {
    uuid: uuid::Uuid,
    pack_id: u16,
    pack_kind: u8,
    pack_size: u64,
    pack_path: PathBuf,
    check_info: CheckInfo,
}

impl PackInfo {
    pub fn bytes(&self, check_info_pos: u64) -> Vec<u8> {
        let mut data = vec![];
        data.extend(self.uuid.as_bytes());
        data.extend(self.pack_size.to_le_bytes());
        data.extend(check_info_pos.to_le_bytes());
        data.extend(self.pack_id.to_le_bytes());
        data.push(self.pack_kind);
        data.push(0); // pack_group
        data.extend(&[0; 2]); // free data id
        let path_data = self.pack_path.to_str().unwrap();
        data.extend((path_data.len() as u8).to_le_bytes());
        data.extend(path_data.as_bytes());
        data.extend(vec![0; 252 - data.len()]);
        data.extend_from_slice(&[0; 4]); // Dummy Crc32
        data
    }

    pub fn get_check_size(&self) -> u64 {
        self.check_info.size()
    }
}

test_suite! {
    name basic_reading;

    use jubako::reader as reader;
    use jubako::reader::{Range, EntryTrait};
    use std::fs::OpenOptions;
    use std::io::{Write, Seek, SeekFrom, Result, Read};
    use std::io;
    use std::path::PathBuf;
    use crate::{Entry as TestEntry, Cluster, KeyStore, IndexStore, Index, PackInfo, CheckInfo};
    use uuid::Uuid;

    fixture compression() -> jubako::CompressionType {
        setup(&mut self) {
            jubako::CompressionType::None
        }
    }

    fixture articles() -> Vec<TestEntry> {
        setup(&mut self) {
            vec![
                TestEntry{
                    path: "foo".to_string(),
                    content: "foo".to_string(),
                    word_count: 1},
                TestEntry{
                    path: "bar".to_string(),
                    content: "foo bar".to_string(),
                    word_count: 2
                }
            ]
        }
    }

    fn create_content_pack(compression: jubako::CompressionType, entries:&Vec<TestEntry>) -> Result<PackInfo> {
        let mut content_pack_path = std::env::temp_dir();
        content_pack_path.push("contentPack.jbkc");
        let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&content_pack_path)?;

        // Write pack header
        file.write_all(&[
            0x6a, 0x62, 0x6b, 0x63,
            0x00, 0x00, 0x00, 0x01,
            0x00, 0x02,
        ])?;
        let uuid = Uuid::new_v4();
        file.write_all(uuid.as_bytes())?;
        file.write_all(&[0x00;6])?; // padding
        file.write_all(&[0x00;16])?; // file size and checksum pos, to be write after
        file.write_all(&[0x00;12])?; // reserved
        file.write_all(&[0; 4])?; // Dummy Crc32

        // Write content pack header
        file.write_all(&0x80_u64.to_le_bytes())?; // entry_ptr_offset
        file.write_all(&0x8C_u64.to_le_bytes())?; // cluster_ptr_offset
        file.write_all(&(entries.len() as u32).to_le_bytes())?; // entry count
        file.write_all(&1_u32.to_le_bytes())?; // cluster count
        file.write_all(&[0xff;36])?; // free_data
        file.write_all(&[0; 4])?; // Dummy Crc32

        // Offse 128/0x80
        // Write entry infos
        file.write_all(&[0x00, 0x00, 0x00, 0x00])?; // first entry info
        file.write_all(&[0x01, 0x00, 0x00, 0x00])?; // second entry info
        file.write_all(&[0; 4])?; // Dummy Crc32

        // Offset 128 + 12 = 140/0x8C
        let cluster_ptr_info_offset = file.seek(SeekFrom::Current(0))?;
        file.write_all(&[0x00;8])?; // cluster offset, to be write after
        file.write_all(&[0; 4])?; // Dummy Crc32

        let mut cluster = Cluster::new(compression, &entries);
        // Cluster data 140 + 12 = 152/0x98
        file.write_all(&cluster.data_bytes())?;

        // Cluster tail
        let cluster_info_offset = file.seek(SeekFrom::Current(0))?.to_le_bytes();
        file.write_all(&cluster.tail_bytes())?;

        // Write back info about where cluster is
        file.seek(SeekFrom::Start(cluster_ptr_info_offset))?;
        file.write_all(&cluster.tail_size().to_le_bytes())?; // write the cluster offset
        file.write_all(&cluster_info_offset[..6])?;

        // Write back info about file_size and checksum_pos
        let checksum_pos = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(32))?;
        file.write_all(&(checksum_pos+33+4+64).to_le_bytes())?;
        file.write_all(&checksum_pos.to_le_bytes())?;

        file.seek(SeekFrom::Start(0))?;
        let mut hasher = blake3::Hasher::new();
        io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_all(&[0x01])?;
        file.write_all(hash.as_bytes())?;
        file.write_all(&[0; 4])?; // Dummy Crc32

        // Write footer
        file.seek(SeekFrom::Start(0))?;
        let mut footer = [0;64];
        file.read_exact(&mut footer)?;
        footer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&footer)?;

        let pack_size = file.seek(SeekFrom::End(0))?;
        Ok(PackInfo{
            check_info: CheckInfo {
                kind: 1,
                data: hash.as_bytes().to_vec(),
            },
            pack_path: content_pack_path,
            pack_size,
            pack_id: 1,
            pack_kind: b'c',
            uuid,
        })
    }

    fn create_directory_pack(entries: &Vec<TestEntry>) -> Result<PackInfo> {
        let mut directory_pack_path = std::env::temp_dir();
        directory_pack_path.push("directoryPack.jbkd");
        let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&directory_pack_path)?;
        // Pack header
        file.write_all(&[
            0x6a, 0x62, 0x6b, 0x64,
            0x00, 0x00, 0x00, 0x01,
            0x00, 0x02,
        ])?;
        let uuid = Uuid::new_v4();
        file.write_all(uuid.as_bytes())?;
        file.write_all(&[0x00;6])?; // padding
        file.write_all(&[0x00;16])?; // file size and checksum pos, to be write after
        file.write_all(&[0x00;12])?; // reserved
        file.write_all(&[0x00; 4])?; // Dummy CRC32

        // Directory pack header
        file.write_all(&[0x00;24])?; // index_ptr_offset, entry_store_ptr_offset, key_store_ptr_offset, to be write after
        file.write_all(&1_u32.to_le_bytes())?; // index count
        file.write_all(&1_u32.to_le_bytes())?; // entry_store count
        file.write_all(&1_u8.to_le_bytes())?; // value_store count
        file.write_all(&[0xff;27])?; // free_data
        file.write_all(&[0; 4])?; // Crc32

        let value_store_ptr_offset = {
            // Write Value Store
            let mut key_store = KeyStore::new(entries);
            file.write_all(&key_store.data_bytes())?;
            let key_store_offset = file.seek(SeekFrom::Current(0))?.to_le_bytes();
            file.write_all(&key_store.tail_bytes())?;
            // Write value store ptr array
            let key_store_ptr_offset = file.seek(SeekFrom::Current(0))?;
            file.write_all(&key_store.tail_size().to_le_bytes())?;
            file.write_all(&key_store_offset[..6])?;
            file.write_all(&[0; 4])?; // Dummy Crc32
            key_store_ptr_offset
        };

        let index_store_ptr_offset = {
            // Write Entry store
            let mut index_store = IndexStore::new(entries);
            file.write_all(&index_store.data_bytes())?;
            let index_store_offset = file.seek(SeekFrom::Current(0))?.to_le_bytes();
            file.write_all(&index_store.tail_bytes())?;

            // Write entry store ptr array
            let index_store_ptr_offset = file.seek(SeekFrom::Current(0))?;
            file.write_all(&index_store.tail_size().to_le_bytes())?;
            file.write_all(&index_store_offset[..6])?;
            file.write_all(&[0; 4])?; // Dummy Crc32
            index_store_ptr_offset
        };

        let index_ptr_offset = {
            // Write index
            let mut index = Index::new(entries);
            let index_offset = file.seek(SeekFrom::Current(0))?.to_le_bytes();
            file.write_all(&index.bytes())?;

            // Write index ptr array
            let index_ptr_offset = file.seek(SeekFrom::Current(0))?;
            file.write_all(&index.tail_size().to_le_bytes())?;
            file.write_all(&index_offset[..6])?;
            file.write_all(&[0; 4])?; // Dummy Crc32
            index_ptr_offset
        };

        let checksum_pos = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(32))?;
        file.write_all(&(checksum_pos+33+4+64).to_le_bytes())?;
        file.write_all(&checksum_pos.to_le_bytes())?;
        file.seek(SeekFrom::Start(64))?;
        file.write_all(&index_ptr_offset.to_le_bytes())?;
        file.write_all(&index_store_ptr_offset.to_le_bytes())?;
        file.write_all(&value_store_ptr_offset.to_le_bytes())?;

        file.seek(SeekFrom::Start(0))?;
        let mut hasher = blake3::Hasher::new();
        io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_all(&[0x01])?;
        file.write_all(hash.as_bytes())?;
        file.write_all(&[0; 4])?; // Dummy Crc32

        // Write footer
        file.seek(SeekFrom::Start(0))?;
        let mut footer = [0;64];
        file.read_exact(&mut footer)?;
        footer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&footer)?;

        let pack_size = file.seek(SeekFrom::End(0))?;
        Ok(PackInfo{
            check_info: CheckInfo {
                kind: 1,
                data: hash.as_bytes().to_vec(),
            },
            pack_path: directory_pack_path,
            pack_size,
            pack_id: 0,
            pack_kind: b'd',
            uuid,
        })
    }

    fn create_main_pack(directory_pack: PackInfo, content_pack:PackInfo) -> Result<PathBuf> {
        let mut manifest_pack_path = std::env::temp_dir();
        manifest_pack_path.push("manifestPack.jbkm");
        let uuid = Uuid::new_v4();
        let mut file_size:u64 = 128;
        let directory_check_info_pos = file_size;
        file_size += directory_pack.get_check_size();
        let content_check_info_pos = file_size;
        file_size += content_pack.get_check_size();
        file_size += 2*256;
        let check_info_pos = file_size;
        file_size += 33+4+64;
        let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&manifest_pack_path)?;

        // Pack header
        file.write_all(&[
            0x6a, 0x62, 0x6b, 0x6d,
            0x00, 0x00, 0x00, 0x01,
            0x00, 0x02,
        ])?;
        file.write_all(uuid.as_bytes())?;
        file.write_all(&[0x00;6])?; // padding
        file.write_all(&file_size.to_le_bytes())?;
        file.write_all(&check_info_pos.to_le_bytes())?;
        file.write_all(&[0x00;12])?; // reserved
        file.write_all(&[0; 4])?; // Dummy Crc32

        // Manifest header
        file.write_all(&[0x02, 0x00])?; // number of pack
        file.write_all(&[0x00; 8])?; // Value store offset
        file.write_all(&[0xff;50])?; // free_data
        file.write_all(&[0; 4])?; // Dummy Crc32

        assert_eq!(directory_check_info_pos, file.seek(SeekFrom::End(0))?);
        file.write_all(&directory_pack.check_info.bytes())?;
        file.write_all(&content_pack.check_info.bytes())?;

        let pack_offset = file.seek(SeekFrom::Current(0))?;
        file.write_all(&directory_pack.bytes(directory_check_info_pos))?;
        file.write_all(&content_pack.bytes(content_check_info_pos))?;

        file.seek(SeekFrom::Start(0))?;
        let mut hasher = blake3::Hasher::new();
        io::copy(&mut Read::by_ref(&mut file).take(pack_offset), &mut hasher)?;
        for _i in 0..2 {
            io::copy(&mut Read::by_ref(&mut file).take(38), &mut hasher)?;
            io::copy(&mut io::repeat(0).take(214+4), &mut hasher)?; // fill with 0 the path
            file.seek(SeekFrom::Current(214+4))?;
        }
        io::copy(&mut file, &mut hasher)?; // finish
        let hash = hasher.finalize();
        file.write_all(&[0x01])?;
        file.write_all(hash.as_bytes())?;
        file.write_all(&[0; 4])?; // Dummy Crc32

        // Write footer
        file.seek(SeekFrom::Start(0))?;
        let mut footer = [0;64];
        file.read_exact(&mut footer)?;
        footer.reverse();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&footer)?;

        Ok(manifest_pack_path)
    }

    test test_content_pack(compression, articles) {
        let content_info = create_content_pack(compression.val, &articles.val).unwrap();
        let directory_info = create_directory_pack(&articles.val).unwrap();
        let main_path = create_main_pack(directory_info, content_info).unwrap();
        let container = reader::Container::new(main_path).unwrap();
        assert_eq!(container.pack_count(), 2.into());
        assert!(container.check().unwrap());
        let index = container.get_index_for_name("Super index").unwrap();
        let builder = reader::builder::AnyBuilder::new(
            index.get_store(&container.get_entry_storage()).unwrap(),
            container.get_value_storage().as_ref()
        ).unwrap();
        assert_eq!(index.count(), (articles.val.len() as u32).into());
        for i in index.count() {
            let entry = index.get_entry(&builder, i).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value_0 = entry.get_value("V0").unwrap();
            let vec = value_0.as_vec().unwrap();
            assert_eq!(vec, articles.val[i.into_u32() as usize].path.as_bytes());
            let value_1 = entry.get_value("V1").unwrap();
            if let reader::RawValue::Content(content) = value_1 {
                assert_eq!(
                    content,
                    jubako::ContentAddress{pack_id:1.into(), content_id:jubako::ContentIdx::from(i.into_u32())}
                );
                let bytes = container.get_bytes(content).unwrap();
                let mut stream = bytes.as_ref().unwrap().stream();
                let mut read_content: String = "".to_string();
                stream.read_to_string(&mut read_content).unwrap();
                assert_eq!(read_content, articles.val[i.into_u32() as usize].content);
            } else {
              panic!();
            }
            let value_2= entry.get_value("V2").unwrap();
            if let reader::RawValue::U16(v) = value_2 {
                assert_eq!(v, articles.val[i.into_u32() as usize].word_count);
            } else {
              panic!();
            }
        }
    }
}
