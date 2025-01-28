use core::convert::TryFrom;
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::Range;
use jubako as jbk;
use std::error::Error;

// We first have to define the entry type we expect to read from jubako container.
// The entry has two variants. We duplicate the common part in the variants for
// simplification.
pub enum Entry {
    Variant0(Variant0),
    Variant1(Variant1),
}

#[derive(Copy, Clone)]
pub enum VariantType {
    Variant0,
    Variant1,
}

impl TryFrom<&str> for VariantType {
    type Error = ();
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        match v {
            "FirstVariant" => Ok(Self::Variant0),
            "SecondVariant" => Ok(Self::Variant1),
            _ => Err(()),
        }
    }
}

pub struct Variant0 {
    value0: Vec<u8>,
    value1: u64,
    value2: jbk::ContentAddress,
}

pub struct Variant1 {
    value0: Vec<u8>,
    value1: u64,
    value2: u64,
}

// The builder is what will build the entry from the data stored in jubako.
// It is a composition of different individual property builder provided by jubako.
pub struct Builder {
    store: jbk::reader::EntryStore,
    variant_id: jbk::reader::builder::VariantIdBuilder<VariantType>,
    value0: jbk::reader::builder::ArrayProperty,
    value1: jbk::reader::builder::IntProperty,
    variant0_value2: jbk::reader::builder::ContentProperty,
    variant1_value2: jbk::reader::builder::IntProperty,
}

// Let's create our builder from the entryStore and ValueStore found in Jubako container.
// This is where we check that the entrystore layout correspond to what we expect.
fn create_builder(
    store: jbk::reader::EntryStore,
    value_storage: &jbk::reader::ValueStorage,
) -> jbk::Result<Builder> {
    let layout = store.layout();
    let variants = layout.variant_part.as_ref().unwrap();
    assert_eq!(layout.variant_len(), 2);
    let value0 = layout.common["AString"]
        .as_builder(value_storage)?
        .expect("Layout proprety should match ArrayProperty");
    let value1 = layout.common["AInteger"]
        .as_builder(value_storage)?
        .expect("Layout proprety should match IntProperty");
    let variant0_value2 = variants.get("FirstVariant").unwrap()["TheContent"]
        .as_builder(value_storage)?
        .expect("Layout proprety should match ContentProperty");
    let variant1_value2 = variants.get("SecondVariant").unwrap()["AnotherInt"]
        .as_builder(value_storage)?
        .expect("Layout proprety should match IntProperty");
    let variant_id = layout.variant_id_builder().unwrap();
    Ok(Builder {
        store,
        value0,
        value1,
        variant0_value2,
        variant1_value2,
        variant_id,
    })
}

// This is where we build our entry
impl jbk::reader::builder::BuilderTrait for Builder {
    type Entry = Entry;
    type Error = jbk::Error;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Option<Self::Entry>> {
        // With this, we can read the bytes corresponding to our entry in the container.
        let reader = self.store.get_entry_reader(idx);
        if reader.is_none() {
            return Ok(None);
        }
        let reader = reader.unwrap();

        // Read the common part

        // Value0 is a array with a part stored in a value store.
        // The property builder only parse the bytes in the entry_store
        // so we have to "resolve" the property to get the data from the value_store.
        let mut value0 = vec![];
        self.value0.create(&reader)?.resolve_to_vec(&mut value0)?;

        // Read value1
        let value1 = self.value1.create(&reader)?;

        // Read other property, depending of the variant.
        match self.variant_id.create(&reader)? {
            Some(VariantType::Variant0) => {
                let value2 = self.variant0_value2.create(&reader)?;
                Ok(Some(Entry::Variant0(Variant0 {
                    value0,
                    value1,
                    value2,
                })))
            }
            Some(VariantType::Variant1) => {
                let value2 = self.variant1_value2.create(&reader)?;
                Ok(Some(Entry::Variant1(Variant1 {
                    value0,
                    value1,
                    value2,
                })))
            }
            None => Ok(None),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Let's read our container created in `simple_create.rs`
    let container = jbk::reader::Container::new("test.jbkm")?; // or "test.jbkm"
    let index = container
        .get_index_for_name("My own index")?
        .expect("'My own index' is in the container.");
    let builder = create_builder(
        index.get_store(container.get_entry_storage())?,
        container.get_value_storage(),
    )?;

    // Now we can read our entries.
    {
        let entry = index
            .get_entry(&builder, 0.into())?
            .expect("Entry 0 exists in the index");
        if let Entry::Variant0(entry) = entry {
            assert_eq!(entry.value0, Vec::from("Super"));
            assert_eq!(entry.value1, 50);
            // Let's print the content on stdout
            let reader = container
                .get_bytes(entry.value2)?
                .and_then(|m| m.transpose())
                .expect("value2 should be valid")
                .unwrap();
            std::io::copy(&mut reader.stream(), &mut std::io::stdout().lock())?;
        } else {
            panic!("We should have variant0")
        }
    }

    {
        let entry = index
            .get_entry(&builder, 1.into())?
            .expect("Entry 1 exists in the index");
        if let Entry::Variant1(entry) = entry {
            assert_eq!(entry.value0, Vec::from("Mega"));
            assert_eq!(entry.value1, 42);
            assert_eq!(entry.value2, 5);
        } else {
            panic!("We should have variant1")
        }
    }

    {
        let entry = index
            .get_entry(&builder, 2.into())?
            .expect("Entry 2 exists in the index");
        if let Entry::Variant1(entry) = entry {
            assert_eq!(entry.value0, Vec::from("Hyper"));
            assert_eq!(entry.value1, 45);
            assert_eq!(entry.value2, 2);
        } else {
            panic!("We should have variant1")
        }
    }

    Ok(())
}
