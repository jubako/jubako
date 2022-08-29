use galvanic_test::test_suite;

struct Entry {
    path: String,
    content: String,
    word_count: u16,
}

test_suite! {
    name basic_creation;

    use jubako::creator;
    use jubako::Result;
    use std::io::Read;
    use crate::Entry;
    use typenum::{U31, U40, U63};

    fixture compression(c: jubako::CompressionType) -> jubako::CompressionType {
        params {
            vec![
                jubako::CompressionType::None,
                jubako::CompressionType::Lz4,
                jubako::CompressionType::Lzma,
                jubako::CompressionType::Zstd,
            ].into_iter()
        }
        setup(&mut self) {
            *self.c
        }
    }

    fixture key_store_kind(k: creator::KeyStoreKind) -> creator::KeyStoreKind {
        params {
            vec![
                creator::KeyStoreKind::Plain,
                creator::KeyStoreKind::Indexed,
            ].into_iter()
        }
        setup(&mut self) {
            *self.k
        }
    }

    fixture articles() -> Vec<Entry> {
        setup(&mut self) {
            vec![
                Entry{
                    path: "foo".to_string(),
                    content: "foo".to_string(),
                    word_count: 1},
                Entry{
                    path: "bar".to_string(),
                    content: "foo bar".to_string(),
                    word_count: 2
                }
            ]
        }
    }

    fn create_content_pack(compression: jubako::CompressionType, entries:&Vec<Entry>) -> Result<creator::PackInfo> {
        let mut creator = creator::ContentPackCreator::new(
            "/tmp/contentPack.jbkc",
            jubako::Id(1),
            1,
            jubako::FreeData::<U40>::clone_from_slice(&[0xff; 40]),
            compression
        );
        creator.start()?;
        for entry in entries {
            let content = entry.content.clone().into_bytes();
            let mut stream = creator::BufStream::new(content, jubako::End::None);
            creator.add_content(&mut stream)?;
        }
        let pack_info = creator.finalize()?;
        Ok(pack_info)
    }

    fn create_directory_pack(key_store_kind: creator::KeyStoreKind, entries: &Vec<Entry>) -> Result<creator::PackInfo> {
        let mut creator = creator::DirectoryPackCreator::new(
            "/tmp/directoryPack.jbkd",
            jubako::Id(1),
            1,
            jubako::FreeData::<U31>::clone_from_slice(&[0xff; 31])
        );
        let key_store_handle = creator.create_key_store(key_store_kind);
        let entry_def = creator::Entry::new(
            vec![
                creator::Variant::new(vec![
                    creator::Key::PString(0, key_store_handle),
                    creator::Key::ContentAddress,
                    creator::Key::UnsignedInt(2)
                ])
            ]
        );
        let entry_store_idx = creator.create_entry_store(entry_def);
        let entry_store = creator.get_entry_store(entry_store_idx);
        for (idx, entry) in entries.iter().enumerate() {
            entry_store.add_entry(0, vec![
                creator::Value::Array{data:entry.path.clone().into(), key_id:None},
                creator::Value::Content(jubako::ContentAddress::new(1.into(), (idx as u32).into())),
                creator::Value::Unsigned(entry.word_count.into())]
            );
        }

        creator.create_index(
            "Super index",
            jubako::ContentAddress::new(0.into(), 0.into()),
            0.into(),
            entry_store_idx,
            jubako::Count(entries.len() as u32),
            jubako::Idx(0));
        let pack_info = creator.finalize().unwrap();
        Ok(pack_info)
    }

    fn create_main_pack(directory_pack: creator::PackInfo, content_pack:creator::PackInfo) -> Result<String> {
        let mut creator = creator::ManifestPackCreator::new(
            "/tmp/mainPack.jbkm",
            1,
            jubako::FreeData::<U63>::clone_from_slice(&[0xff; 63])
        );

        creator.add_pack(directory_pack);
        creator.add_pack(content_pack);
        creator.finalize()?;
        Ok("/tmp/mainPack.jbkm".to_string())
    }



    test test_content_pack(compression, key_store_kind, articles) {
        let content_info = create_content_pack(compression.val, &articles.val).unwrap();
        let directory_info = create_directory_pack(key_store_kind.val, &articles.val).unwrap();
        let main_path = create_main_pack(directory_info, content_info).unwrap();

        let container = jubako::reader::Container::new(main_path).unwrap();
        assert_eq!(container.pack_count(), jubako::Count(1));
        assert!(container.check().unwrap());
        println!("Read directory pack");
        let directory_pack = container.get_directory_pack().unwrap();
        let index = directory_pack.get_index(jubako::Idx(0)).unwrap();
        println!("Read index");
        assert_eq!(index.entry_count().0, articles.val.len() as u32);
        for i in 0..index.entry_count().0 {
            println!("Check entry count {}", i);
            let entry = index.get_entry(jubako::Idx(i)).unwrap();
            assert_eq!(entry.get_variant_id(), 0);
            println!("Check value 0");
            let value_0 = entry.get_value(jubako::Idx(0)).unwrap();
            if let jubako::reader::Value::Array(array) = value_0 {
                let vec = array.resolve_to_vec(&directory_pack.get_key_storage()).unwrap();
                assert_eq!(vec, articles.val[i as usize].path.as_bytes());
            } else {
              panic!();
            }
            println!("Check value 1");
            let value_1 = entry.get_value(jubako::Idx(1)).unwrap();
            if let jubako::reader::Value::Content(content) = value_1 {
                println!("Get reader");
                let reader = container.get_reader(content).unwrap();
                println!("Readir is {:?}", reader);
                let mut stream = reader.create_stream_all();
                println!("Stream is {:?}", stream);
                let mut read_content: String = "".to_string();
                println!("Read from stream");
                stream.read_to_string(&mut read_content).unwrap();
                assert_eq!(read_content, articles.val[i as usize].content);
            } else {
              panic!();
            }
            println!("Check value 2");
            let value_2= entry.get_value(jubako::Idx(2)).unwrap();
            if let jubako::reader::Value::U16(v) = value_2 {
                assert_eq!(*v, articles.val[i as usize].word_count);
            } else {
              panic!();
            }
        }
    }
}
