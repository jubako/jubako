use jbk::creator::EntryStoreTrait;
use jubako::creator::{schema, EntryTrait};
use jubako::{self as jbk};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

// This is what will allow Jubako to differenciate your format from others.
const VENDOR_ID: jbk::VendorId = jbk::VendorId::new([1, 2, 3, 4]);

// Let's use a static str as property identifier.
// More complex application may want to use a enum instead.
type PropertyName = &'static str;

// Let's use a static str as variant identifier.
// More complex application may want to use a enum instead.
type VariantName = &'static str;

// Entries in a entry store have a fixed size. So strings (which have variable size) must be store elsewhere.
// This elsewhere is a ValueStore.
struct CustomEntryStore {
    value_store: jbk::creator::StoreHandle,
    schema: schema::Schema<PropertyName, VariantName>,
    entry_store: Vec<SimpleEntry>,
}

struct SimpleEntry {
    variant_name: VariantName,
    values: HashMap<PropertyName, jbk::Value>,
}

impl EntryTrait<PropertyName, VariantName> for SimpleEntry {
    fn variant_name(&self) -> Option<VariantName> {
        Some(self.variant_name)
    }

    fn value(&self, name: &PropertyName) -> jbk::Value {
        self.values[name].clone()
    }

    fn value_count(&self) -> jubako::PropertyCount {
        jbk::PropertyCount::from(self.values.len() as u8)
    }
}

impl CustomEntryStore {
    fn new() -> Self {
        // Entries have fixed sizes. We need to store variable length values in an extra store.
        let value_store = jbk::creator::ValueStore::new_plain(None);

        // Let's define our entry schema. We will have two variants (named `FirstVariant` and `SecondVariant`).
        // Variants will have two properties in common (`AString` and `AInteger`).
        let schema = schema::Schema::new(
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

        let entry_store = Vec::new();

        Self {
            value_store,
            schema,
            entry_store,
        }
    }

    fn add_entry(&mut self, entry: SimpleEntry) {
        // We have to create a EntryType from our values.
        // To do so, we would have to preprocess the values :
        // - add the `AString` value to the value_store and store only the idx of the value in the value store.
        // - Transform from `jbk::Value` to `jbk::creator::Value`.
        // - Provide a entry id.
        // - Be sure that values match the properties declared in the schema for the given property
        // Hopefully, `new_from_schema` does this for us.
        // It panics if values don't match the schema/variant.
        self.entry_store.push(entry);
    }
}

impl EntryStoreTrait for CustomEntryStore {
    fn finalize(self: Box<Self>, directory_pack: &mut jbk::creator::DirectoryPackCreator) {
        // We have to populate the DirectoryPack with our data.

        // First, we can add our (unique here) value store.
        directory_pack.add_value_store(self.value_store);

        // Then, add our (unique here) entry store.
        let entry_store = jbk::creator::EntryStore::new(self.schema, self.entry_store);
        let entry_store_id = directory_pack.add_entry_store(Box::new(entry_store));

        // We have to reference (a entry range in) our entry store to lets readers find it.
        // This is done with a "Index"
        directory_pack.create_index(
            "My own index", // This is the name of our index. Reader will seach for it.
            Default::default(),
            0.into(), // The index is not sorted
            entry_store_id,
            3.into(),                  // Our index is 3 entries length
            jubako::EntryIdx::from(0), // starting at offset 0
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Let's create a basic creator. It wrapper a ContentCreator and
    // correctly write files, manifest packs... at finish.
    let mut creator = jbk::creator::BasicCreator::new(
        "test.jbk",
        jbk::creator::ConcatMode::OneFile, // Let's put all our packs in one file
        VENDOR_ID,
        jbk::creator::Compression::default(),
        Arc::new(()),
    )?;

    // The store for our entries.
    let mut entry_store = Box::new(CustomEntryStore::new());

    // Now we have "configured" our creator, let's add some content:
    let content: Vec<u8> = "A super content prime quality for our test container".into();
    let content_address =
        creator.add_content(Box::new(std::io::Cursor::new(content)), Default::default())?;
    entry_store.add_entry(SimpleEntry {
        variant_name: "FirstVariant",
        values: HashMap::from([
            ("AString", jbk::Value::Array("Super".into())),
            ("AInteger", jbk::Value::Unsigned(50)),
            ("TheContent", jbk::Value::Content(content_address)),
        ]),
    });

    entry_store.add_entry(SimpleEntry {
        variant_name: "SecondVariant",
        values: HashMap::from([
            ("AString", jbk::Value::Array("Mega".into())),
            ("AInteger", jbk::Value::Unsigned(42)),
            ("AnotherInt", jbk::Value::Unsigned(5)),
        ]),
    });

    entry_store.add_entry(SimpleEntry {
        variant_name: "SecondVariant",
        values: HashMap::from([
            ("AString", jbk::Value::Array("Hyper".into())),
            ("AInteger", jbk::Value::Unsigned(45)),
            ("AnotherInt", jbk::Value::Unsigned(2)),
        ]),
    });

    Ok(creator.finalize(entry_store, vec![])?)

    // You have now 3 files : "test.jbkm", "test.jbkc" and "test.jbkd".

    // Let's concat them in only one.
    //jbk::concat(&["test.jbkm", "test.jbkc", "test.jbkd"], "test.jbk")?;
    // We have now 4 files. The 4th is "test.jbk" and it contains the 3 others.
}
