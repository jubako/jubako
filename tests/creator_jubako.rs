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
    use jubako::reader::{Range, EntryTrait};
    use std::io::Read;
    use crate::Entry as TestEntry;
    use std::collections::HashMap;

    fixture compression(c: jubako::CompressionType) -> jubako::CompressionType {
        params {
            vec![
                jubako::CompressionType::None,
                #[cfg(feature="lz4")]
                jubako::CompressionType::Lz4,
                #[cfg(feature="lzma")]
                jubako::CompressionType::Lzma,
                #[cfg(feature="zstd")]
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
        )?;
        for entry in entries {
            let content = entry.content.clone().into_bytes();
            creator.add_content(content.into())?;
        }
        let pack_info = creator.finalize(Some("/tmp/contentPack.jbkc".into()))?;
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
        let entry_def = schema::Schema::new::<String>
        (
            schema::CommonProperties::new(vec![
                schema::Property::new_array(0, key_store_handle, "V0"),
                schema::Property::new_content_address("V1"),
                schema::Property::new_uint("V2")
            ]),
            vec!(),
            None
        );

        let mut entry_store = Box::new(creator::EntryStore::new(entry_def));
        for (idx, entry) in entries.iter().enumerate() {
            entry_store.add_entry(creator::BasicEntry::new_from_schema(&entry_store.schema, None, HashMap::from([
                ("V0", jubako::Value::Array(entry.path.clone().into())),
                ("V1", jubako::Value::Content(jubako::ContentAddress::new(1.into(), (idx as u32).into()))),
                ("V2", jubako::Value::Unsigned((entry.word_count as u64).into()))
            ])));
        }

        let entry_store_idx = creator.add_entry_store(entry_store);
        creator.create_index(
            "Super index",
            jubako::ContentAddress::new(0.into(), 0.into()),
            0.into(),
            entry_store_idx,
            (entries.len() as u32).into(),
            jubako::EntryIdx::from(0).into());
        let pack_info = creator.finalize(Some("/tmp/directoryPack.jbkd".into())).unwrap();
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
        let builder = jubako::reader::builder::AnyBuilder::new(
            index.get_store(&entry_storage).unwrap(),
            value_storage.as_ref()
        ).unwrap();
        println!("Read index");
        assert_eq!(index.count(), (articles.val.len() as u32).into());
        for i in index.count() {
            println!("Check entry count {:?}", i);
            let entry = index.get_entry(&builder, i.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            println!("Check value 0");
            let value_0 = entry.get_value("V0").unwrap();
            println!("Raw value 0 is {:?}", value_0);
            let value_0 = value_0.as_vec().unwrap();
            assert_eq!(value_0, articles.val[i.into_usize()].path.as_bytes());
            println!("Check value 1");
            let value_1 = entry.get_value("V1").unwrap();
            println!("Raw value 1 is {:?}", value_1);
            let value_1 = value_1.as_content();
            println!("Value 1 is {:?}", value_1);
            println!("Get reader");
            let reader = container.get_reader(value_1).unwrap();
            println!("Readir is {:?}", reader);
            let mut flux = reader.create_flux_all();
            println!("flux is {:?}", flux);
            let mut read_content: String = "".to_string();
            println!("Read from flux");
            flux.read_to_string(&mut read_content).unwrap();
            assert_eq!(read_content, articles.val[i.into_usize()].content);
            println!("Check value 2");
            let value_2 = entry.get_value("V2").unwrap();
            let value_2 = value_2.as_unsigned();
            assert_eq!(value_2, articles.val[i.into_usize()].word_count as u64);
        }
    }
}
