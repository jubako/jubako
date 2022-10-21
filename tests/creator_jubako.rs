use galvanic_test::test_suite;

struct Entry {
    path: String,
    content: String,
    word_count: u16,
}

test_suite! {
    name basic_creation;

    use jubako::creator;
    use jubako::creator::layout;
    use jubako::Result;
    use jubako::reader::EntryTrait;
    use std::io::Read;
    use std::rc::Rc;
    use crate::Entry as TestEntry;
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

    fixture key_store_kind(k: creator::ValueStoreKind) -> creator::ValueStoreKind {
        params {
            vec![
                creator::ValueStoreKind::Plain,
                creator::ValueStoreKind::Indexed,
            ].into_iter()
        }
        setup(&mut self) {
            *self.k
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
                    word_count: 256
                }
            ]
        }
    }

    fn create_content_pack(compression: jubako::CompressionType, entries:&Vec<TestEntry>) -> Result<creator::PackInfo> {
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

    fn create_directory_pack(key_store_kind: creator::ValueStoreKind, entries: &Vec<TestEntry>) -> Result<creator::PackInfo> {
        let mut creator = creator::DirectoryPackCreator::new(
            "/tmp/directoryPack.jbkd",
            jubako::Id(1),
            1,
            jubako::FreeData::<U31>::clone_from_slice(&[0xff; 31])
        );
        let key_store_handle = creator.create_value_store(key_store_kind);
        let entry_def = layout::Entry::new(
            vec![
                layout::Variant::new(vec![
                    layout::Property::VLArray(0, key_store_handle),
                    layout::Property::ContentAddress,
                    layout::Property::new_int()
                ])
            ]
        );
        let entry_store_idx = creator.create_entry_store(entry_def);
        let entry_store = creator.get_entry_store(entry_store_idx);
        for (idx, entry) in entries.iter().enumerate() {
            entry_store.add_entry(0, vec![
                creator::Value::Array(entry.path.clone().into()),
                creator::Value::Content(creator::Content::new(jubako::ContentAddress::new(1.into(), (idx as u32).into()), None)),
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
        let resolver = directory_pack.get_resolver();
        let finder = index.get_finder(Rc::clone(&resolver));
        println!("Read index");
        assert_eq!(index.entry_count().0, articles.val.len() as u32);
        for i in 0..index.entry_count().0 {
            println!("Check entry count {}", i);
            let entry = finder.get_entry(jubako::Idx(i)).unwrap();
            assert_eq!(entry.get_variant_id(), 0);
            println!("Check value 0");
            let value_0 = entry.get_value(jubako::Idx(0)).unwrap();
            let value_0 = resolver.resolve_to_vec(&value_0).unwrap();
            assert_eq!(value_0, articles.val[i as usize].path.as_bytes());
            println!("Check value 1");
            let value_1 = entry.get_value(jubako::Idx(1)).unwrap();
            let value_1 = resolver.resolve_to_content(&value_1);
            println!("Get reader");
            let reader = container.get_reader(value_1).unwrap();
            println!("Readir is {:?}", reader);
            let mut stream = reader.create_stream_all();
            println!("Stream is {:?}", stream);
            let mut read_content: String = "".to_string();
            println!("Read from stream");
            stream.read_to_string(&mut read_content).unwrap();
            assert_eq!(read_content, articles.val[i as usize].content);
            println!("Check value 2");
            let value_2 = entry.get_value(jubako::Idx(2)).unwrap();
            let value_2 = resolver.resolve_to_unsigned(&value_2);
            assert_eq!(value_2, articles.val[i as usize].word_count as u64);
        }
    }
}
