use jubako as jbk;
use jubako::creator::schema;
use std::error::Error;
use std::rc::Rc;

// This is what will allow Jubako to differenciate your format from others.
const VENDOR_ID: u32 = 0x01_02_03_04;

fn main() -> Result<(), Box<dyn Error>> {
    let mut content_pack = jbk::creator::ContentPackCreator::new(
        "test.jbkc",
        jbk::PackId::from(1), // The pack id as referenced in the container
        VENDOR_ID,
        jbk::FreeData40::clone_from_slice(&[0x00; 40]), // Put whatever you what, this is for you
        jbk::CompressionType::Zstd,                     // How to compress
    )?;

    let mut directory_pack = jbk::creator::DirectoryPackCreator::new(
        "test.jbkd",
        jbk::PackId::from(0),
        VENDOR_ID,
        jbk::FreeData31::clone_from_slice(&[0x00; 31]),
    );

    // Entries have fixed sizes. We need to store variable length values in an extra store.
    let value_store = directory_pack.create_value_store(jbk::creator::ValueStoreKind::Plain);

    // Our entry kind will have two variants.
    let entry_def = schema::Schema::new(
        schema::CommonProperties::new(vec![
            schema::Property::new_array(0, Rc::clone(&value_store)), // One string, will be stored in value_store
            schema::Property::new_int(),                             // A integer
        ]),
        vec![
            schema::VariantProperties::new(vec![
                schema::Property::new_content_address(), // A "pointer" to a content.
            ]),
            schema::VariantProperties::new(vec![
                schema::Property::new_int(), //
            ]),
        ],
    );

    // The store for our entries.
    let mut entry_store = Box::new(jbk::creator::EntryStore::new(entry_def));

    // Now we have "configured" our container, let's add some content:
    let content: Vec<u8> = "A super content prime quality for our test container".into();
    let content_id = content_pack.add_content(content.into())?;
    entry_store.add_entry(jbk::creator::BasicEntry::new_from_schema(
        &entry_store.schema,
        Some(0.into()), // Variant 0
        vec![
            jbk::Value::Array("Super".into()),
            jbk::Value::Unsigned(50.into()),
            jbk::Value::Content(jbk::ContentAddress::new(
                jbk::PackId::from(1), // Pack id
                content_id,           // Content id in the pack
            )),
        ],
    ));

    entry_store.add_entry(jbk::creator::BasicEntry::new_from_schema(
        &entry_store.schema,
        Some(1.into()), // Variant 1
        vec![
            jbk::Value::Array("Mega".into()),
            jbk::Value::Unsigned(42.into()),
            jbk::Value::Unsigned(5.into()),
        ],
    ));

    entry_store.add_entry(jbk::creator::BasicEntry::new_from_schema(
        &entry_store.schema,
        Some(1.into()), // Variant 1
        vec![
            jbk::Value::Array("Hyper".into()),
            jbk::Value::Unsigned(45.into()),
            jbk::Value::Unsigned(2.into()),
        ],
    ));

    let entry_store_id = directory_pack.add_entry_store(entry_store);

    // One index to access our entries.
    directory_pack.create_index(
        "My own index",
        jubako::ContentAddress::new(0.into(), 0.into()), // A pointer to a content which can be used to store whatever you what (nothing here)
        0.into(),                                        // The index is not sorted
        entry_store_id,
        3.into(), // 3 entries
        0.into(), // Offset 0
    );

    let directory_pack_info = directory_pack.finalize()?;
    let content_pack_info = content_pack.finalize()?;
    let mut manifest_creator = jbk::creator::ManifestPackCreator::new(
        "test.jbkm",
        VENDOR_ID,
        jbk::FreeData63::clone_from_slice(&[0x00; 63]),
    );

    manifest_creator.add_pack(directory_pack_info);
    manifest_creator.add_pack(content_pack_info);
    manifest_creator.finalize()?;
    // You have now 3 files : "test.jbkm", "test.jbkc" and "test.jbkd".

    // Let's concat them in only one.
    jbk::concat(&["test.jbkm", "test.jbkc", "test.jbkd"], "test.jbk")?;
    // We have now 4 files. The 4th is "test.jbk" and it contains the 3 others.

    Ok(())
}
