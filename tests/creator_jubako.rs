use galvanic_test::test_suite;

struct Entry {
    path: String,
    content: String,
    word_count: u16,
}

test_suite! {
    name basic_creation;

    use jubako::creator;
    use jubako::creator::schema;
    use jubako::Result;
    use jubako::reader::EntryTrait;
    use jubako::reader::schema::SchemaTrait;
    use std::io::Read;
    use crate::Entry as TestEntry;

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

    fn create_content_pack(compression: jubako::CompressionType, entries:&Vec<TestEntry>) -> Result<creator::PackData> {
        let mut creator = creator::ContentPackCreator::new(
            "/tmp/contentPack.jbkc",
            jubako::PackId::from(1),
            1,
            jubako::FreeData40::clone_from_slice(&[0xff; 40]),
            compression
        );
        creator.start()?;
        for entry in entries {
            let content = entry.content.clone().into_bytes();
            let mut stream = creator::Stream::new(content, jubako::End::None);
            creator.add_content(&mut stream)?;
        }
        let pack_info = creator.finalize()?;
        Ok(pack_info)
    }

    fn create_directory_pack(key_store_kind: creator::ValueStoreKind, entries: &Vec<TestEntry>) -> Result<creator::PackData> {
        let mut creator = creator::DirectoryPackCreator::new(
            "/tmp/directoryPack.jbkd",
            jubako::PackId::from(1),
            1,
            jubako::FreeData31::clone_from_slice(&[0xff; 31])
        );
        let key_store_handle = creator.create_value_store(key_store_kind);
        let entry_def = schema::Schema::new(
            schema::CommonProperties::new(vec![
                schema::Property::VLArray(0, key_store_handle),
                schema::Property::ContentAddress,
                schema::Property::new_int()
            ]),
            vec!()
        );

        let mut entry_store = Box::new(creator::EntryStore::new(entry_def));
        for (idx, entry) in entries.iter().enumerate() {
            entry_store.add_entry(creator::BasicEntry::new(&entry_store.schema, None, vec![
                creator::Value::Array(entry.path.clone().into()),
                creator::Value::Content(jubako::ContentAddress::new(1.into(), (idx as u32).into())),
                creator::Value::Unsigned(entry.word_count.into())]
            ));
        }

        let entry_store_idx = creator.add_entry_store(entry_store);
        creator.create_index(
            "Super index",
            jubako::ContentAddress::new(0.into(), 0.into()),
            0.into(),
            entry_store_idx,
            (entries.len() as u32).into(),
            0.into());
        let pack_info = creator.finalize().unwrap();
        Ok(pack_info)
    }

    fn create_main_pack(directory_pack: creator::PackData, content_pack:creator::PackData) -> Result<String> {
        let mut creator = creator::ManifestPackCreator::new(
            "/tmp/mainPack.jbkm",
            1,
            jubako::FreeData63::clone_from_slice(&[0xff; 63])
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
        assert_eq!(container.pack_count(), 1.into());
        assert!(container.check().unwrap());
        println!("Read directory pack");
        let directory_pack = container.get_directory_pack();
        let index = directory_pack.get_index(0.into()).unwrap();
        let entry_storage = directory_pack.create_entry_storage();
        let value_storage = directory_pack.create_value_storage();
        let resolver = jubako::reader::Resolver::new(value_storage);
        let schema = jubako::reader::AnySchema {};
        let builder = schema.create_builder(index.get_store(&entry_storage).unwrap()).unwrap();
        let finder: jubako::reader::Finder<jubako::reader::AnySchema> = index.get_finder(&builder).unwrap();
        println!("Read index");
        assert_eq!(index.entry_count(), (articles.val.len() as u32).into());
        for i in index.entry_count() {
            println!("Check entry count {:?}", i);
            let entry = finder.get_entry(i.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            println!("Check value 0");
            let value_0 = entry.get_value(0.into()).unwrap();
            let value_0 = resolver.resolve_to_vec(&value_0).unwrap();
            assert_eq!(value_0, articles.val[i.into_usize()].path.as_bytes());
            println!("Check value 1");
            let value_1 = entry.get_value(1.into()).unwrap();
            let value_1 = resolver.resolve_to_content(&value_1);
            println!("Get reader");
            let reader = container.get_reader(value_1).unwrap();
            println!("Readir is {:?}", reader);
            let mut stream = reader.create_stream_all();
            println!("Stream is {:?}", stream);
            let mut read_content: String = "".to_string();
            println!("Read from stream");
            stream.read_to_string(&mut read_content).unwrap();
            assert_eq!(read_content, articles.val[i.into_usize()].content);
            println!("Check value 2");
            let value_2 = entry.get_value(2.into()).unwrap();
            let value_2 = resolver.resolve_to_unsigned(&value_2);
            assert_eq!(value_2, articles.val[i.into_usize()].word_count as u64);
        }
    }
}
