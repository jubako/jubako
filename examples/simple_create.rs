use jubako as jbk;
use jubako::creator::schema;
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;

// This is what will allow Jubako to differenciate your format from others.
const VENDOR_ID: u32 = 0x01_02_03_04;

fn main() -> Result<(), Box<dyn Error>> {
    let mut content_pack = jbk::creator::ContentPackCreator::new(
        "test.jbkc",
        jbk::PackId::from(1), // The pack id as referenced in the container
        VENDOR_ID,
        Default::default(), // Put whatever you what, this is for you
        jbk::creator::Compression::default(), // How to compress
    )?;

    let mut directory_pack = jbk::creator::DirectoryPackCreator::new(
        jbk::PackId::from(0),
        VENDOR_ID,
        Default::default(),
    );

    // Entries have fixed sizes. We need to store variable length values in an extra store.
    let value_store = jbk::creator::ValueStore::new_plain(None);

    // Our entry kind will have two variants.
    let entry_def = schema::Schema::new(
        schema::CommonProperties::new(vec![
            schema::Property::new_array(0, value_store.clone(), "AString"), // One string, will be stored in value_store
            schema::Property::new_uint("AInteger"),                         // A integer
        ]),
        vec![
            (
                "FirstVariant",
                schema::VariantProperties::new(vec![
                    schema::Property::new_content_address("TheContent"), // A "pointer" to a content.
                ]),
            ),
            (
                "SecondVariant",
                schema::VariantProperties::new(vec![schema::Property::new_uint("AnotherInt")]),
            ),
        ],
        None,
    );

    // The store for our entries.
    let mut entry_store = Box::new(jbk::creator::EntryStore::new(entry_def, None));

    // Now we have "configured" our container, let's add some content:
    let content: Vec<u8> = "A super content prime quality for our test container".into();
    let content = std::io::Cursor::new(content);
    let content_id = content_pack.add_content(content)?;
    entry_store.add_entry(jbk::creator::BasicEntry::new_from_schema(
        &entry_store.schema,
        Some("FirstVariant"), // Variant 0
        HashMap::from([
            ("AString", jbk::Value::Array("Super".into())),
            ("AInteger", jbk::Value::Unsigned(50)),
            (
                "TheContent",
                jbk::Value::Content(jbk::ContentAddress::new(
                    jbk::PackId::from(1), // Pack id
                    content_id,           // Content id in the pack
                )),
            ),
        ]),
    ));

    entry_store.add_entry(jbk::creator::BasicEntry::new_from_schema(
        &entry_store.schema,
        Some("SecondVariant"),
        HashMap::from([
            ("AString", jbk::Value::Array("Mega".into())),
            ("AInteger", jbk::Value::Unsigned(42)),
            ("AnotherInt", jbk::Value::Unsigned(5)),
        ]),
    ));

    entry_store.add_entry(jbk::creator::BasicEntry::new_from_schema(
        &entry_store.schema,
        Some("SecondVariant"),
        HashMap::from([
            ("AString", jbk::Value::Array("Hyper".into())),
            ("AInteger", jbk::Value::Unsigned(45)),
            ("AnotherInt", jbk::Value::Unsigned(2)),
        ]),
    ));

    directory_pack.add_value_store(value_store);
    let entry_store_id = directory_pack.add_entry_store(entry_store);

    // One index to access our entries.
    directory_pack.create_index(
        "My own index",
        Default::default(),
        0.into(), // The index is not sorted
        entry_store_id,
        3.into(),                         // 3 entries
        jubako::EntryIdx::from(0).into(), // Offset 0
    );

    let mut directory_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open("test.jbkd")?;
    let directory_pack_info = directory_pack.finalize(&mut directory_file)?;
    let (_file, content_pack_info) = content_pack.finalize()?;
    let mut manifest_creator =
        jbk::creator::ManifestPackCreator::new(VENDOR_ID, Default::default());

    manifest_creator.add_pack(directory_pack_info, "test.jbkd".into());
    manifest_creator.add_pack(content_pack_info, "test.jbkc".into());

    let mut manifest_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open("test.jbkm")?;
    manifest_creator.finalize(&mut manifest_file)?;
    // You have now 3 files : "test.jbkm", "test.jbkc" and "test.jbkd".

    // Let's concat them in only one.
    //jbk::concat(&["test.jbkm", "test.jbkc", "test.jbkd"], "test.jbk")?;
    // We have now 4 files. The 4th is "test.jbk" and it contains the 3 others.

    Ok(())
}
