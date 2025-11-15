use rustest::{test, *};
struct Entry {
    path: String,
    content: String,
    word_count: u16,
}

use crate::Entry as TestEntry;
use camino::{Utf8Path, Utf8PathBuf};
use jubako::creator;
use jubako::creator::{schema, Result};
use jubako::reader::{EntryTrait, Range};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum ValueStoreKindParam {
    Plain,
    Indexed,
}

impl rustest::ParamName for ValueStoreKindParam {
    fn param_name(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Clone)]

pub struct CompressionParam(creator::Compression);

impl rustest::ParamName for CompressionParam {
    fn param_name(&self) -> String {
        format!("{:?}", self.0)
    }
}

#[fixture(params: CompressionParam=[
    CompressionParam(creator::Compression::None),
    #[cfg(feature="lz4")]
    CompressionParam(creator::Compression::lz4()),
    #[cfg(feature="lzma")]
    CompressionParam(creator::Compression::lzma()),
    #[cfg(feature="zstd")]
    CompressionParam(creator::Compression::zstd()),
])]
fn Compression(Param(c): Param) -> creator::Compression {
    c.0
}

#[fixture(params: ValueStoreKindParam=[
    ValueStoreKindParam::Plain,
    ValueStoreKindParam::Indexed,
])]
fn ValueStoreKind(Param(k): Param) -> ValueStoreKindParam {
    k
}

#[fixture]
fn Articles() -> Vec<TestEntry> {
    vec![
        TestEntry {
            path: "foo".to_string(),
            content: "foo".to_string(),
            word_count: 1,
        },
        TestEntry {
            path: "bar".to_string(),
            content: "foo bar".to_string(),
            word_count: 256,
        },
    ]
}

fn create_content_pack(
    compression: creator::Compression,
    entries: &Vec<TestEntry>,
    outfile: &Utf8Path,
) -> Result<(creator::PackData, jubako::Reader)> {
    let mut creator = creator::ContentPackCreator::new(
        outfile,
        jubako::PackId::from(1),
        jubako::VendorId::from([1, 0, 0, 0]),
        Default::default(),
        compression,
    )?;
    for entry in entries {
        let content = entry.content.clone();
        let content = Box::new(std::io::Cursor::new(content));
        creator.add_content(content, Default::default())?;
    }
    let (mut file, pack_info) = creator.finalize()?;
    file.rewind()?;
    Ok((
        pack_info,
        jubako::FileSource::new(file.into_inner())?.into(),
    ))
}

fn create_directory_pack(
    value_store_kind: ValueStoreKindParam,
    entries: &[TestEntry],
    outfile: &Path,
) -> Result<(creator::PackData, jubako::Reader)> {
    let mut creator = creator::DirectoryPackCreator::new(
        jubako::PackId::from(1),
        jubako::VendorId::from([1, 0, 0, 0]),
        Default::default(),
    );
    let value_store = match value_store_kind {
        ValueStoreKindParam::Plain => creator::ValueStore::new_plain(None),
        ValueStoreKindParam::Indexed => creator::ValueStore::new_indexed(),
    };
    creator.add_value_store(value_store.clone());
    let mut entry_def = schema::Schema::<&str, &str>::new(
        schema::CommonProperties::new(vec![
            schema::Property::new_array(0, value_store, "V0"),
            schema::Property::new_content_address("V1"),
            schema::Property::new_uint("V2"),
        ]),
        vec![],
        None,
    );

    let mut entry_store = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        entry_store.push(creator::BasicEntry::new_from_schema(
            &mut entry_def,
            None,
            HashMap::from([
                ("V0", jubako::Value::Array(entry.path.as_bytes().into())),
                (
                    "V1",
                    jubako::Value::Content(jubako::ContentAddress::new(
                        1.into(),
                        (idx as u32).into(),
                    )),
                ),
                ("V2", jubako::Value::Unsigned(entry.word_count as u64)),
            ]),
        ));
    }

    let entry_store = jubako::creator::EntryStore::new(entry_def, entry_store);
    let entry_store_idx = creator.add_entry_store(Box::new(entry_store));
    creator.create_index(
        "Super index",
        Default::default(),
        0.into(),
        entry_store_idx,
        (entries.len() as u32).into(),
        jubako::EntryIdx::from(0),
    );

    let mut directory_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(outfile)?;
    let pack_info = creator.finalize()?.write(&mut directory_file).unwrap();
    directory_file.rewind().unwrap();
    Ok((
        pack_info,
        jubako::FileSource::new(directory_file).unwrap().into(),
    ))
}

fn create_main_pack(
    directory_pack: creator::PackData,
    content_pack: creator::PackData,
    temp_dir: &Path,
) -> Result<PathBuf> {
    let mut creator =
        creator::ManifestPackCreator::new(jubako::VendorId::from([1, 0, 0, 0]), Default::default());

    creator.add_pack(directory_pack, "directoryPack.jbkd");
    creator.add_pack(content_pack, "contentPack.jbkc");

    let mut manifest_path = temp_dir.to_path_buf();
    manifest_path.push("manifestPath.jbkm");
    let mut main_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&manifest_path)?;

    let uuid = creator.finalize(&mut main_file)?;
    println!("manifest uuid: {uuid}");
    Ok(manifest_path)
}

struct Locator(pub HashMap<uuid::Uuid, jubako::Reader>);

impl Locator {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn add(&mut self, uuid: uuid::Uuid, reader: jubako::Reader) {
        self.0.insert(uuid, reader);
    }
}

impl jubako::reader::PackLocatorTrait for Locator {
    fn locate(&self, uuid: uuid::Uuid, _helper: &str) -> jubako::Result<Option<jubako::Reader>> {
        println!("Search for {uuid}");
        println!("We have {:?}", self.0);
        Ok(self.0.get(&uuid).cloned())
    }
}

#[test]
fn test_content_pack(
    compression: Compression,
    value_store_kind: ValueStoreKind,
    articles: Articles,
    temp_dir: rustest_fixtures::TempDir,
) {
    println!(
        "%%%%%%%%%%%%%%%%%%%%%%\nTesting in {}",
        temp_dir.path().display()
    );
    let utf8_temp_dir: Utf8PathBuf = temp_dir
        .path()
        .to_path_buf()
        .try_into()
        .expect("We expect an utf8 path");
    let mut content_pack_path = utf8_temp_dir.clone();
    content_pack_path.push("contentPack.jbkc");
    let (content_info, content_reader) =
        create_content_pack(*compression, &articles, &content_pack_path).unwrap();
    let mut directory_pack_path = temp_dir.path().to_path_buf();
    directory_pack_path.push("directoryPack.jbkd");
    let (directory_info, directory_reader) =
        create_directory_pack(*value_store_kind, &articles, &directory_pack_path).unwrap();

    let mut locator = Locator::new();
    println!("content_info.uuid: {}", content_info.uuid);
    println!("directory_info.uuid: {}", directory_info.uuid);
    locator.add(content_info.uuid, content_reader);
    locator.add(directory_info.uuid, directory_reader);

    let main_path = create_main_pack(directory_info, content_info, temp_dir.path()).unwrap();

    let container =
        jubako::reader::Container::new_with_locator(main_path, Arc::new(locator)).unwrap();
    assert_eq!(container.pack_count(), 2.into());
    assert!(container.check().unwrap());
    println!("Read directory pack");
    let directory_pack = container.get_directory_pack();
    let index = directory_pack.get_index(0.into()).unwrap();
    let entry_storage = directory_pack.create_entry_storage();
    let value_storage = directory_pack.create_value_storage();
    let builder = jubako::reader::builder::AnyBuilder::new(
        index.get_store(&entry_storage).unwrap(),
        value_storage.as_ref(),
    )
    .unwrap();
    println!("Read index");
    assert_eq!(index.count(), (articles.len() as u32).into());
    for i in index.count() {
        println!("Check entry count {:?}", i);
        let entry = index
            .get_entry(&builder, i)
            .unwrap()
            .expect("Entry i is in the index");
        assert_eq!(entry.get_variant_id().unwrap(), None);
        println!("Check value 0");
        let value_0 = entry.get_value("V0").unwrap().unwrap();
        println!("Raw value 0 is {:?}", value_0);
        let value_0 = value_0.as_vec().unwrap();
        assert_eq!(value_0, articles[i.into_usize()].path.as_bytes());
        println!("Check value 1");
        let value_1 = entry.get_value("V1").unwrap().unwrap();
        println!("Raw value 1 is {:?}", value_1);
        let value_1 = value_1.as_content();
        println!("Value 1 is {:?}", value_1);
        println!("Get reader");
        let bytes = container.get_bytes(value_1).unwrap();
        println!("Readir is {:?}", bytes);
        let mut stream = bytes
            .and_then(|m| m.transpose())
            .expect("value_1 should be valid")
            .unwrap()
            .stream();
        let mut read_content: String = "".to_string();
        println!("Read from stream");
        stream.read_to_string(&mut read_content).unwrap();
        assert_eq!(read_content, articles[i.into_usize()].content);
        println!("Check value 2");
        let value_2 = entry.get_value("V2").unwrap().unwrap();
        let value_2 = value_2.as_unsigned();
        assert_eq!(value_2, articles[i.into_usize()].word_count as u64);
    }
}

#[main]
fn main() {}
