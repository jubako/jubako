use galvanic_test::test_suite;

struct Entry {
    path: String,
    content: String,
    word_count: u16,
}

test_suite! {
    name basic_reading;

    use jubako::creator as creator;
    use std::io::{Result, Read};
    use crate::Entry;
    use typenum::{U31, U40, U63};

    fixture compression() -> jubako::CompressionType {
        setup(&mut self) {
            jubako::CompressionType::None
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
            1,
            1,
            jubako::FreeData::<U40>::clone_from_slice(&[0xff; 40])
        );
        creator.start()?;
        for entry in entries {
            creator.add_content(entry.content.as_bytes())?;
        }
        let pack_info = creator.finalize()?;
        Ok(pack_info)
    }

    fn create_directory_pack(entries: &Vec<Entry>) -> Result<creator::PackInfo> {
        let mut creator = creator::DirectoryPackCreator::new(
            "/tmp/directoryPack.jbkd",
            1,
            1,
            jubako::FreeData::<U31>::clone_from_slice(&[0xff; 31])
        );
        let key_store_handle = creator.create_key_store();
        let entryDef = creator::Entry::new(
            vec![
                creator::Variant::new(vec![
                    creator::Key::PString(0, key_store_handle),
                    creator::Key::ContentAddress,
                    creator::Key::UnsignedInt(2)
                ])
            ]
        );
        let entry_store_handle = creator.create_entry_store(entryDef);
        for (idx, entry) in entries.iter().enumerate() {
            entry_store_handle.get_mut().add_entry(0, vec![
                creator::Value::Array{data:entry.path.clone().into(), key_id:None},
                creator::Value::Content(jubako::ContentAddress::new(0.into(), (idx as u32).into())),
                creator::Value::Unsigned(entry.word_count.into())]
            );
        }

        let entry_store_idx = entry_store_handle.get().get_idx();
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
        let mut creator = creator::MainPackCreator::new(
            "/tmp/mainPack.jbkm",
            1,
            jubako::FreeData::<U63>::clone_from_slice(&[0xff; 63])
        );

        creator.add_pack(directory_pack);
        creator.add_pack(content_pack);
        creator.finalize()?;
        Ok("/tmp/mainPack.jbkm".to_string())
    }



    test test_content_pack(compression, articles) {
        let content_info = create_content_pack(compression.val, &articles.val).unwrap();
        let directory_info = create_directory_pack(&articles.val).unwrap();
        let main_path = create_main_pack(directory_info, content_info).unwrap();

        let container = jubako::Container::new(main_path).unwrap();
        assert_eq!(container.pack_count(), 1);
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
            if let jubako::Value::Array(array) = value_0 {
                assert_eq!(
                    array,
                    &jubako::Array::new(
                        vec!(),
                        Some(jubako::Extend::new(jubako::Idx(0), i.into()))
                    ));
                let key_store = directory_pack.get_key_store(jubako::Idx(0)).unwrap();
                let vec = array.resolve_to_vec(&key_store).unwrap();
                assert_eq!(vec, articles.val[i as usize].path.as_bytes());
            } else {
              panic!();
            }
            println!("Check value 1");
            let value_1 = entry.get_value(jubako::Idx(1)).unwrap();
            if let jubako::Value::Content(content) = value_1 {
                assert_eq!(
                    content,
                    &jubako::Content::new(
                        jubako::ContentAddress{pack_id:0.into(), content_id:i.into()},
                        None
                    ));
                println!("Get pack");
                let pack = container.get_pack(1.into()).unwrap();
                println!("Get reader");
                let reader = pack.get_content(i.into()).unwrap();
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
            if let jubako::Value::U16(v) = value_2 {
                assert_eq!(*v, articles.val[i as usize].word_count);
            } else {
              panic!();
            }
        }
    }
}
