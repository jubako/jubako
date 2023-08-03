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
    use std::sync::Arc;
    use std::fs::OpenOptions;

    #[derive(Clone, Copy, Debug)]
    pub enum ValueStoreKind {
        Plain,
        Indexed,
    }

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

    fixture value_store_kind(k: ValueStoreKind) -> ValueStoreKind {
        params {
            vec![
                ValueStoreKind::Plain,
                ValueStoreKind::Indexed,
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

    fn create_content_pack(compression: jubako::CompressionType, entries:&Vec<TestEntry>) -> Result<(creator::PackData, jubako::Reader)> {
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
        let (file, pack_info) = creator.finalize()?;
        Ok((pack_info, jubako::FileSource::new(file)?.into()))
    }

    fn create_directory_pack(value_store_kind: ValueStoreKind, entries: &Vec<TestEntry>) -> Result<(creator::PackData, jubako::Reader)> {
        let mut creator = creator::DirectoryPackCreator::new(
            jubako::PackId::from(1),
            1,
            jubako::FreeData31::clone_from_slice(&[0xff; 31])
        );
        let value_store = match value_store_kind {
            ValueStoreKind::Plain => creator::ValueStore::new_plain(),
            ValueStoreKind::Indexed => creator::ValueStore::new_indexed()
        };
        creator.add_value_store(value_store.clone());
        let entry_def = schema::Schema::<&str, &str>::new(
            schema::CommonProperties::new(vec![
                schema::Property::new_array(0, value_store, "V0"),
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

        let mut directory_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open("/tmp/directoryPack.jbkd")?;
        let pack_info = creator.finalize(&mut directory_file).unwrap();
        Ok((pack_info, jubako::FileSource::open("/tmp/directoryPack.jbkd").unwrap().into()))
    }

    fn create_main_pack(directory_pack: creator::PackData, content_pack:creator::PackData) -> Result<String> {
        let mut creator = creator::ManifestPackCreator::new(
            1,
            jubako::FreeData55::clone_from_slice(&[0xff; 55])
        );

        creator.add_pack(directory_pack, "/tmp/directoryPack.jbkd".into());
        creator.add_pack(content_pack, "/tmp/contentPack.jbkc".into());

        let mut main_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open("/tmp/mainPack.jbkm")?;

        let uuid = creator.finalize(&mut main_file)?;
        println!("manifest uuid: {uuid}");
        Ok("/tmp/mainPack.jbkm".to_string())
    }

    struct Locator(pub HashMap<uuid::Uuid, jubako::Reader>);

    impl Locator {
        pub fn new() -> Self {
            Self (Default::default())
        }

        pub fn add(&mut self, uuid: uuid::Uuid, reader: jubako::Reader) {
            self.0.insert(uuid, reader);
        }
    }

    impl jubako::reader::PackLocatorTrait for Locator {
        fn locate(&self, uuid: uuid::Uuid, _helper: &[u8]) -> jubako::Result<Option<jubako::Reader>> {
            println!("Search for {uuid}");
            println!("We have {:?}", self.0);
            Ok(self.0.get(&uuid).cloned())
        }
    }



    test test_content_pack(compression, value_store_kind, articles) {
        let (content_info, content_reader) = create_content_pack(compression.val, &articles.val).unwrap();
        let (directory_info, directory_reader) = create_directory_pack(value_store_kind.val, &articles.val).unwrap();

        let mut locator = Locator::new();
        println!("content_info.uuid: {}", content_info.uuid);
        println!("directory_info.uuid: {}", directory_info.uuid);
        locator.add(content_info.uuid, content_reader);
        locator.add(directory_info.uuid, directory_reader);

        let main_path = create_main_pack(directory_info, content_info).unwrap();


        let container = jubako::reader::Container::new_with_locator(main_path, Arc::new(locator)).unwrap();
        assert_eq!(container.pack_count(), 2.into());
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
