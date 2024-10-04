use jubako as jbk;
use jubako::creator::schema;
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;

// This is what will allow Jubako to differenciate your format from others.
const VENDOR_ID: jbk::VendorId = jbk::VendorId::new([1, 2, 3, 4]);

fn main() -> Result<(), Box<dyn Error>> {
    // We need a contentPack creator to store our content.
    let mut content_pack = jbk::creator::ContentPackCreator::new(
        "test.jbkc",
        jbk::PackId::from(1), // The pack id as referenced in the container
        VENDOR_ID,
        Default::default(), // Put whatever you what, this is for you
        jbk::creator::Compression::default(), // How to compress
    )?;

    // We need a directoryPack creator to store our directory (information about the entries).
    let mut directory_pack = jbk::creator::DirectoryPackCreator::new(
        jbk::PackId::from(0),
        VENDOR_ID,
        Default::default(),
    );

    // Entries have fixed sizes. We need to store variable length values in an extra store.
    let value_store = jbk::creator::ValueStore::new_plain(None);

    // Let's define our entry schema. We will have two variants (named `FirstVariant` and `SecondVariant`).
    // Variants will have two properties in common (`AString` and `AInteger`).
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

    // Now we have "configured" our creator, let's add some entries:

    // For the first entry, we have a content, we need to add it to our conten creator.
    let content: Vec<u8> = "A super content prime quality for our test container".into();
    let content = Box::new(std::io::Cursor::new(content));
    let content_address = content_pack.add_content(content, Default::default())?;

    // Now it is added, we can add the entry itself.
    // We have to create a Entry from our values.
    // To do so, we would have to preprocess the values :
    // - add the `AString` value to the value_store and store only the idx of the value in the value store.
    // - Transform from `jbk::Value` to `jbk::creator::Value`.
    // - Provide a entry id.
    // - Be sure that values match the properties declared in the schema for the given property
    // Hopefully, `new_from_schema` does this for us.
    // It panics if values don't match the schema/variant.
    entry_store.add_entry(jbk::creator::BasicEntry::new_from_schema(
        &entry_store.schema,
        Some("FirstVariant"), // Variant 0
        HashMap::from([
            ("AString", jbk::Value::Array("Super".into())),
            ("AInteger", jbk::Value::Unsigned(50)),
            ("TheContent", jbk::Value::Content(content_address)),
        ]),
    ));

    // Now we add our two other entries. We don't have content in the second variant
    // so we can directly add the entries to the entry_ store.
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

    // We have added all our content/entries.
    // Time to finish the creation process.

    // Add the value store and the entry store the directory.
    directory_pack.add_value_store(value_store);
    let entry_store_id = directory_pack.add_entry_store(entry_store);

    // We have to reference (a entry range in) our entry store to lets readers find it.
    // This is done with a "Index"
    directory_pack.create_index(
        "My own index", // This is the name of our index. Reader will seach for it.
        Default::default(),
        0.into(), // The index is not sorted
        entry_store_id,
        3.into(),                         // Our index is 3 entries length
        jubako::EntryIdx::from(0).into(), // starting at offset 0
    );

    // Let's write the directory pack in "test.jbkd" file
    let mut directory_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open("test.jbkd")?;
    let directory_pack_info = directory_pack.finalize()?.write(&mut directory_file)?;

    // Let's finalize content pack creation.
    // We don't care about returned file as we will not store the content pack in a container.
    let (_file, content_pack_info) = content_pack.finalize()?;

    // Let's start the creation of the manifest.
    // The manifest is the entry point to find other packs. It must list add least a directory pack
    // and optionally some content pack.
    let mut manifest_creator =
        jbk::creator::ManifestPackCreator::new(VENDOR_ID, Default::default());

    // As we don't store packs in a container, we have to indicate where to find the directory pack.
    manifest_creator.add_pack(directory_pack_info, "test.jbkd".into());

    // As we don't store packs in a container, we have to indicate where to find the content pack.
    manifest_creator.add_pack(content_pack_info, "test.jbkc".into());

    let mut manifest_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open("test.jbkm")?;
    manifest_creator.finalize(&mut manifest_file)?;
    // You have now 3 files : "test.jbkm", "test.jbkc" and "test.jbkd".

    Ok(())
}
