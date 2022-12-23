use jubako as jbk;
use jubako::creator::layout;
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
    );
    content_pack.start()?;

    let mut directory_pack = jbk::creator::DirectoryPackCreator::new(
        "test.jbkd",
        jbk::PackId::from(0),
        VENDOR_ID,
        jbk::FreeData31::clone_from_slice(&[0x00; 31]),
    );

    // Entries have fixed sizes. We need to store variable length values in an extra store.
    let value_store = directory_pack.create_value_store(jbk::creator::ValueStoreKind::Plain);

    // Our entry kind will have two variants.
    let entry_def = layout::Entry::new(vec![
        layout::Variant::new(vec![
            layout::Property::VLArray(0, Rc::clone(&value_store)), // One string, will be stored in value_store
            layout::Property::new_int(),                           // A integer
            layout::Property::ContentAddress,                      // A "pointer" to a content.
        ]),
        layout::Variant::new(vec![
            layout::Property::VLArray(0, Rc::clone(&value_store)),
            layout::Property::new_int(), //
            layout::Property::new_int(), //
        ]),
    ]);

    // The store for our entries.
    let entry_store_id = directory_pack.create_entry_store(entry_def);
    let entry_store = directory_pack.get_entry_store(entry_store_id);

    // Now we have "configured" our container, let's add some content:
    let content: Vec<u8> = "A super content prime quality for our test container".into();
    let mut reader = jbk::creator::Stream::new(content, jbk::End::None);
    let content_id = content_pack.add_content(&mut reader)?;
    entry_store.add_entry(
        0, // Variant 0
        vec![
            jbk::creator::Value::Array("Super".into()),
            jbk::creator::Value::Unsigned(50),
            jbk::creator::Value::Content(jbk::creator::Content::from((
                jbk::PackId::from(1), // Pack id
                content_id,           // Content id in the pack
            ))),
        ],
    );

    entry_store.add_entry(
        1, // Variant 1
        vec![
            jbk::creator::Value::Array("Mega".into()),
            jbk::creator::Value::Unsigned(42),
            jbk::creator::Value::Unsigned(5),
        ],
    );

    entry_store.add_entry(
        1, // Variant 1
        vec![
            jbk::creator::Value::Array("Hyper".into()),
            jbk::creator::Value::Unsigned(45),
            jbk::creator::Value::Unsigned(2),
        ],
    );

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