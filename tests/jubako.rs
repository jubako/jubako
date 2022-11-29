use galvanic_test::test_suite;

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
        data.extend((self.entries_offset.len() as u16).to_be_bytes()); // blob_count
        data.extend((self.entries_offset[self.entries_offset.len() - 1] as u64).to_be_bytes()); //raw data size
        data.extend((self.data_size.unwrap() as u64).to_be_bytes()); // data size
        for offset in &self.entries_offset[..(self.entries_offset.len() - 1)] {
            data.extend((*offset as u64).to_be_bytes());
        }
        self.tail_size = Some(data.len() as u16);
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
        data
    }

    pub fn tail_bytes(&mut self) -> Vec<u8> {
        let mut data = vec![];
        data.push(0x01); // kind
        data.extend((self.entries_offset.len() as u64).to_be_bytes()); // key count
        data.push(0x08); // offset size [TODO] Use a better size
        data.extend((self.entries_offset[self.entries_offset.len() - 1] as u64).to_be_bytes()); //data size
        for offset in &self.entries_offset[..(self.entries_offset.len() - 1)] {
            data.extend((*offset as u64).to_be_bytes());
        }
        self.tail_size = Some(data.len() as u16);
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
            // - The path : A 0Array/PString
            // - The content : a content address
            // - The words counts : a u16
            data.extend(&[idx].to_vec());
            data.extend(&(idx as u32).to_be_bytes().to_vec());
            data.extend(&entry.word_count.to_be_bytes().to_vec());
            idx += 1;
        }
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
        data.extend(7_u16.to_be_bytes()); // entry_size
        data.push(0x01); // variant count
        data.push(0x03); // key count
        data.extend(&[0b0110_0000, 0x00]); // The first key, a PString(1) idx 0
        data.extend(&[0b0001_0000]); // The second key, the content address
        data.extend(&[0b0010_0001]); // The third key, the u16
        data.extend((self.data.len() as u64).to_be_bytes()); //data size
        self.tail_size = Some(data.len() as u16);
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
        data.extend(self.store_id.to_be_bytes()); // store_id
        data.extend(self.entry_count.to_be_bytes()); // entry_count
        data.extend(0_u32.to_be_bytes()); // entry_offset
        data.extend(0_u32.to_be_bytes()); // extra_data
        data.extend(self.index_key.to_be_bytes()); // index_key
        data.push(self.index_name.len() as u8);
        data.extend(self.index_name.bytes()); // The third key, the u16
        self.tail_size = Some(data.len() as u16);
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
        data
    }

    pub fn size(&self) -> u64 {
        (self.data.len() as u64) + 1
    }
}

struct PackInfo {
    uuid: uuid::Uuid,
    pack_id: u8,
    pack_size: u64,
    pack_path: String,
    check_info: CheckInfo,
}

impl PackInfo {
    pub fn bytes(&self, check_info_pos: u64) -> Vec<u8> {
        let mut data = vec![];
        data.extend(self.uuid.as_bytes());
        data.push(self.pack_id);
        data.extend(&[0; 103]);
        data.extend(self.pack_size.to_be_bytes());
        data.extend(check_info_pos.to_be_bytes());
        data.extend(&[0; 8]); // offest
        let path_data = self.pack_path.as_bytes();
        data.extend((path_data.len() as u8).to_be_bytes());
        data.extend(path_data);
        data.extend(vec![0; 256 - data.len()]);
        data
    }

    pub fn get_check_size(&self) -> u64 {
        self.check_info.size()
    }
}

test_suite! {
    name basic_reading;

    use jubako::reader as reader;
    use jubako::reader::EntryTrait;
    use std::fs::OpenOptions;
    use std::io::{Write, Seek, SeekFrom, Result, Read};
    use std::io;
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
        let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open("/tmp/contentPack.jbkc")?;
        file.write_all(&[
            0x6a, 0x62, 0x6b, 0x63,
            0x00, 0x00, 0x00, 0x01,
            0x00, 0x00,
        ])?;
        let uuid = Uuid::new_v4();
        file.write_all(uuid.as_bytes())?;
        file.write_all(&[0x00;6])?; // padding
        file.write_all(&[0x00;16])?; // file size and checksum pos, to be write after
        file.write_all(&[0x00;16])?; // reserved
        file.write_all(&0x80_u64.to_be_bytes())?; // entry_ptr_offset
        file.write_all(&((0x80+4*entries.len()) as u64).to_be_bytes())?; // cluster_ptr_offset
        file.write_all(&(entries.len() as u32).to_be_bytes())?; // entry count
        file.write_all(&1_u32.to_be_bytes())?; // cluster count
        file.write_all(&[0xff;40])?; // free_data
        file.write_all(&[0x00, 0x00, 0x00, 0x00])?; // first entry info
        file.write_all(&[0x00, 0x00, 0x00, 0x01])?; // second entry info
        let cluster_ptr_info_offset = file.seek(SeekFrom::Current(0))?;
        file.write_all(&[0x00;8])?; // cluster offset.
        let mut cluster = Cluster::new(compression, &entries);
        file.write_all(&cluster.data_bytes())?;
        let cluster_info_offset = file.seek(SeekFrom::Current(0))?.to_be_bytes();
        file.write_all(&cluster.tail_bytes())?;
        file.seek(SeekFrom::Start(cluster_ptr_info_offset))?;
        file.write_all(&cluster.tail_size().to_be_bytes())?;
        file.write_all(&cluster_info_offset[2..])?;
        let checksum_pos = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(32))?;
        file.write_all(&(checksum_pos+33).to_be_bytes())?;
        file.write_all(&checksum_pos.to_be_bytes())?;

        file.seek(SeekFrom::Start(0))?;
        let mut hasher = blake3::Hasher::new();
        io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_all(&[0x01])?;
        file.write_all(hash.as_bytes())?;
        let pack_size = file.seek(SeekFrom::End(0))?;
        Ok(PackInfo{
            check_info: CheckInfo {
                kind: 1,
                data: hash.as_bytes().to_vec(),
            },
            pack_path: "/tmp/contentPack.jbkc".to_string(),
            pack_size,
            pack_id: 1,
            uuid,
        })
    }

    fn create_directory_pack(entries: &Vec<TestEntry>) -> Result<PackInfo> {
        let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open("/tmp/directoryPack.jbkd")?;
        file.write_all(&[
            0x6a, 0x62, 0x6b, 0x64,
            0x01, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ])?;
        let uuid = Uuid::new_v4();
        file.write_all(uuid.as_bytes())?;
        file.write_all(&[0x00;6])?; // padding
        file.write_all(&[0x00;16])?; // file size and checksum pos, to be write after
        file.write_all(&[0x00;16])?; // reserved
        file.write_all(&[0x00;24])?; // index_ptr_offset, entry_store_ptr_offset, key_store_ptr_offset, to be write after
        file.write_all(&1_u32.to_be_bytes())?; // index count
        file.write_all(&1_u32.to_be_bytes())?; // entry_store count
        file.write_all(&1_u8.to_be_bytes())?; // key_store counti
        file.write_all(&[0xff;31])?; // free_data

        let key_store_ptr_offset = {
            let mut key_store = KeyStore::new(entries);
            file.write_all(&key_store.data_bytes())?;
            let key_store_offset = file.seek(SeekFrom::Current(0))?.to_be_bytes();
            file.write_all(&key_store.tail_bytes())?;
            let key_store_ptr_offset = file.seek(SeekFrom::Current(0))?;
            file.write_all(&key_store.tail_size().to_be_bytes())?;
            file.write_all(&key_store_offset[2..])?;
            key_store_ptr_offset
        };

        let index_store_ptr_offset = {
            let mut index_store = IndexStore::new(entries);
            file.write_all(&index_store.data_bytes())?;
            let index_store_offset = file.seek(SeekFrom::Current(0))?.to_be_bytes();
            file.write_all(&index_store.tail_bytes())?;
            let index_store_ptr_offset = file.seek(SeekFrom::Current(0))?;
            file.write_all(&index_store.tail_size().to_be_bytes())?;
            file.write_all(&index_store_offset[2..])?;
            index_store_ptr_offset
        };

        let index_ptr_offset = {
            let mut index = Index::new(entries);
            let index_offset = file.seek(SeekFrom::Current(0))?.to_be_bytes();
            file.write_all(&index.bytes())?;
            let index_ptr_offset = file.seek(SeekFrom::Current(0))?;
            file.write_all(&index.tail_size().to_be_bytes())?;
            file.write_all(&index_offset[2..])?;
            index_ptr_offset
        };
        let checksum_pos = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(32))?;
        file.write_all(&(checksum_pos+33).to_be_bytes())?;
        file.write_all(&checksum_pos.to_be_bytes())?;
        file.seek(SeekFrom::Start(64))?;
        file.write_all(&index_ptr_offset.to_be_bytes())?;
        file.write_all(&index_store_ptr_offset.to_be_bytes())?;
        file.write_all(&key_store_ptr_offset.to_be_bytes())?;

        file.seek(SeekFrom::Start(0))?;
        let mut hasher = blake3::Hasher::new();
        io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        file.write_all(&[0x01])?;
        file.write_all(hash.as_bytes())?;
        let pack_size = file.seek(SeekFrom::End(0))?;
        Ok(PackInfo{
            check_info: CheckInfo {
                kind: 1,
                data: hash.as_bytes().to_vec(),
            },
            pack_path: "/tmp/directoryPack.jbkd".to_string(),
            pack_size,
            pack_id: 0,
            uuid,
        })
    }

    fn create_main_pack(directory_pack: PackInfo, content_pack:PackInfo) -> Result<String> {
        let uuid = Uuid::new_v4();
        let mut file_size:u64 = 128 + 2*256;
        let directory_check_info_pos = file_size;
        file_size += directory_pack.get_check_size();
        let content_check_info_pos = file_size;
        file_size += content_pack.get_check_size();
        let check_info_pos = file_size;
        file_size += 33;
        let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open("/tmp/mainPack.jbkm")?;
        file.write_all(&[
            0x6a, 0x62, 0x6b, 0x6d,
            0x01, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ])?;
        file.write_all(uuid.as_bytes())?;
        file.write_all(&[0x00;6])?; // padding
        file.write_all(&file_size.to_be_bytes())?;
        file.write_all(&check_info_pos.to_be_bytes())?;
        file.write_all(&[0x00;16])?; // reserved
        file.write_all(&[0x01])?; // number of contentpack
        file.write_all(&[0xff;63])?; // free_data

        file.write_all(&directory_pack.bytes(directory_check_info_pos))?;
        file.write_all(&content_pack.bytes(content_check_info_pos))?;
        assert_eq!(directory_check_info_pos, file.seek(SeekFrom::End(0))?);
        file.write_all(&directory_pack.check_info.bytes())?;
        file.write_all(&content_pack.check_info.bytes())?;

        file.seek(SeekFrom::Start(0))?;
        let mut hasher = blake3::Hasher::new();
        let mut buf = [0u8;256];
        file.read_exact(&mut buf[..128])?;
        hasher.write_all(&buf[..128])?; //check start
        for _i in 0..2 {
            file.read_exact(&mut buf[..144])?;
            hasher.write_all(&buf[..144])?; //check beggining of pack
            io::copy(&mut io::repeat(0).take(112), &mut hasher)?; // fill with 0 the path
            file.seek(SeekFrom::Current(112))?;
        }
        io::copy(&mut file, &mut hasher)?; // finish
        let hash = hasher.finalize();
        file.write_all(&[0x01])?;
        file.write_all(hash.as_bytes())?;
        Ok("/tmp/mainPack.jbkm".to_string())
    }



    test test_content_pack(compression, articles) {
        let content_info = create_content_pack(compression.val, &articles.val).unwrap();
        let directory_info = create_directory_pack(&articles.val).unwrap();
        let main_path = create_main_pack(directory_info, content_info).unwrap();
        let container = reader::Container::new(main_path).unwrap();
        assert_eq!(container.pack_count(), 1.into());
        assert!(container.check().unwrap());
        let directory_pack = container.get_directory_pack();
        let index = directory_pack.get_index(0.into()).unwrap();
        let entry_storage = directory_pack.create_entry_storage();
        let value_storage = directory_pack.create_value_storage();
        let resolver = reader::Resolver::new(value_storage);
        let finder = index.get_finder(&entry_storage, resolver.clone()).unwrap();
        assert_eq!(index.entry_count(), (articles.val.len() as u32).into());
        for i in index.entry_count() {
            let entry = finder.get_entry(i).unwrap();
            assert_eq!(entry.get_variant_id(), 0);
            let value_0 = entry.get_value(0.into()).unwrap();
            if let reader::RawValue::Array(array) = &value_0 {
                assert_eq!(
                    array,
                    &reader::testing::Array::new(
                        vec!(),
                        Some(reader::testing::Extend::new(0.into(), jubako::ValueIdx::from(i.into_u64())))
                    ));
                let vec = resolver.resolve_to_vec(&value_0).unwrap();
                assert_eq!(vec, articles.val[i.into_u32() as usize].path.as_bytes());
            } else {
              panic!();
            }
            let value_1 = entry.get_value(1.into()).unwrap();
            if let reader::RawValue::Content(content) = value_1 {
                assert_eq!(
                    content,
                    reader::testing::Content::new(
                        jubako::ContentAddress{pack_id:0.into(), content_id:jubako::ContentIdx::from(i.into_u32())},
                        None
                    ));
                let pack = container.get_pack(1.into()).unwrap();
                let reader = pack.get_content(jubako::ContentIdx::from(i.into_u32())).unwrap();
                let mut stream = reader.create_stream_all();
                let mut read_content: String = "".to_string();
                stream.read_to_string(&mut read_content).unwrap();
                assert_eq!(read_content, articles.val[i.into_u32() as usize].content);
            } else {
              panic!();
            }
            let value_2= entry.get_value(2.into()).unwrap();
            if let reader::RawValue::U16(v) = value_2 {
                assert_eq!(v, articles.val[i.into_u32() as usize].word_count);
            } else {
              panic!();
            }
        }
    }
}
