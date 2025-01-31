use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::Range;
use jubako as jbk;
use std::error::Error;
use std::fmt::Display;

// This example is showing how to create a Jubako reader which expect a specific
// schema.
//
// This reader is expected to read a Jubako file created by `simple_create.rs` or
// `basic_creator.rs`.
// It expects:
//  - An index called "My own index" which point to an entry store with a specific schema:
//  - 2 variants (called "FirstVariant" and "SecondVariant")
//  - 2 common properties (properties common to all variants):
//    . A byte array called "AString"
//    . An integer called "AInteger"
//  - variant 0 ("FirstVariant") has one property which is a content reference called "TheContent"
//  - variant 1 ("SecondVariant") has one property which is a integer called "AnotherInteger"
//
// Note that the Jubako container may contain another metadata
// (other index, other variants or other properties), but we don't care.
// We still can read a Jubako container as long as it is a super set of what expected.

// Jubako API will return Option<T> type with None value if the expected metadata are
// not found. A classical reader will do a `.ok_or` to transform this `Option` to a
// `Result`. While an error is not necessary (you can still work with option), macros
// provided to help implementing a reader is expecting one to transform the `Option`.
// Let's define a error type now.
#[derive(Debug)]
pub struct ExampleError(&'static str);
impl Display for ExampleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for ExampleError {}

// Then we have to declare our two variants.
// This will define a enum `VariantType` with the field `Variant0` and `Variant1`.
// It defines also their name with expect to see in the container.
jbk::variants! {
    VariantType {
        Variant0 => "FirstVariant",
        Variant1 => "SecondVariant"
    }
}

// We have 4 properties: 2 commons and one per variant.
jbk::properties! {
    Property {
        Value0:"array" => "AString",
        Value1:"int" => "AInteger",
        Variant0Value2:"content" => "TheContent",
        Variant1Value2:"int" => "AnotherInteger"
    }
}

// Now we define our Entry type itself.
// This is the library's structure exposed to the user.
// Nothing force us to have only one `Entry` struct.
// Different use case may imply different Entry created from the same schema.
// For exemple, a function listing only the entry may be interseted only by `value0`
// while a function printing the content will have to parse the reader variant type and
// the content reference.
pub struct Entry {
    value0: Vec<u8>,
    value1: u64,
    variant: Variant,
}

pub enum Variant {
    Variant0 { value2: jbk::ContentAddress },
    Variant1 { value2: u64 },
}

// As we may have several kind of entry type (and each library will define its own),
// We need a builder which will build the entry from the data stored in jubako.
// It is a composition of different individual property builder provided by jubako.
pub struct Builder {
    // The store associated to this builder
    store: jbk::reader::EntryStore,

    // The variant id is what give us the actual variant of the entry being read
    variant_id: jbk::reader::builder::VariantIdBuilder<VariantType>,

    // Now, a builder per property.
    value0: jbk::reader::builder::ArrayProperty,
    value1: jbk::reader::builder::IntProperty,
    variant0_value2: jbk::reader::builder::ContentProperty,
    variant1_value2: jbk::reader::builder::IntProperty,
}

// Let's create our builder from the entryStore and ValueStore found in Jubako container.
// This is where we check that the entrystore layout correspond to what we expect.
// The macro `layout_builder` will extract the information about the property in the layout
// and will build a property builder for us.
// If it cannot (missing variant/property or wrong type), it will return a `ExempleError`.
fn create_builder(
    store: jbk::reader::EntryStore,
    value_storage: &jbk::reader::ValueStorage,
) -> Result<Builder, Box<dyn Error>> {
    let layout = store.layout();
    assert_eq!(layout.variant_len(), 2);
    // We can unwrap the variant_id as we know that their is 2 variants.
    let variant_id = layout.variant_id_builder().unwrap();
    let value0 = jbk::layout_builder!(
        layout[common][Property::Value0],
        value_storage,
        ExampleError
    );
    let value1 = jbk::layout_builder!(
        layout[common][Property::Value1],
        value_storage,
        ExampleError
    );
    let variant0_value2 = jbk::layout_builder!(
        layout[VariantType::Variant0][Property::Variant0Value2],
        value_storage,
        ExampleError
    );
    let variant1_value2 = jbk::layout_builder!(
        layout[VariantType::Variant1][Property::Variant1Value2],
        value_storage,
        ExampleError
    );
    Ok(Builder {
        store,
        value0,
        value1,
        variant0_value2,
        variant1_value2,
        variant_id,
    })
}

// The builder now have to create the entry.
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
        let mut value0 = Vec::new();
        self.value0.create(&reader)?.resolve_to_vec(&mut value0)?;

        // Read value1
        let value1 = self.value1.create(&reader)?;

        // Read other property, depending of the variant.
        match self.variant_id.create(&reader)? {
            Some(VariantType::Variant0) => {
                let value2 = self.variant0_value2.create(&reader)?;
                Ok(Some(Entry {
                    value0,
                    value1,
                    variant: Variant::Variant0 { value2 },
                }))
            }
            Some(VariantType::Variant1) => {
                let value2 = self.variant1_value2.create(&reader)?;
                Ok(Some(Entry {
                    value0,
                    value1,
                    variant: Variant::Variant1 { value2 },
                }))
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
        assert_eq!(entry.value0, b"Super");
        assert_eq!(entry.value1, 50);
        if let Variant::Variant0 { value2 } = entry.variant {
            // Let's print the content on stdout
            let reader = container
                .get_bytes(value2)?
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
        assert_eq!(entry.value0, b"Mega");
        assert_eq!(entry.value1, 42);
        if let Variant::Variant1 { value2 } = entry.variant {
            assert_eq!(value2, 5);
        } else {
            panic!("We should have variant1")
        }
    }

    {
        let entry = index
            .get_entry(&builder, 2.into())?
            .expect("Entry 2 exists in the index");
        assert_eq!(entry.value0, b"Hyper");
        assert_eq!(entry.value1, 45);
        if let Variant::Variant1 { value2 } = entry.variant {
            assert_eq!(value2, 2);
        } else {
            panic!("We should have variant1")
        }
    }

    Ok(())
}
