use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::Range;
use jubako as jbk;
use std::error::Error;
use std::sync::Arc;

pub enum Entry {
    Variant0(Variant0),
    Variant1(Variant1),
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

pub struct Builder {
    store: Arc<jbk::reader::EntryStore>,
    variant_id: jbk::reader::builder::VariantIdProperty,
    value0: jbk::reader::builder::ArrayProperty,
    value1: jbk::reader::builder::IntProperty,
    variant0_value2: jbk::reader::builder::ContentProperty,
    variant1_value2: jbk::reader::builder::IntProperty,
}

impl jbk::reader::builder::BuilderTrait for Builder {
    type Entry = Entry;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);

        // Read the common part
        let mut value0 = vec![];
        self.value0.create(&reader)?.resolve_to_vec(&mut value0)?;
        let value1 = self.value1.create(&reader)?;
        match self.variant_id.create(&reader)?.into_u8() {
            0 => {
                let value2 = self.variant0_value2.create(&reader)?;
                Ok(Entry::Variant0(Variant0 {
                    value0,
                    value1,
                    value2,
                }))
            }
            1 => {
                let value2 = self.variant1_value2.create(&reader)?;
                Ok(Entry::Variant1(Variant1 {
                    value0,
                    value1,
                    value2,
                }))
            }
            _ => Err("Unknown variant".into()),
        }
    }
}

fn create_builder(
    store: Arc<jbk::reader::EntryStore>,
    value_storage: &jbk::reader::ValueStorage,
) -> jbk::Result<Builder> {
    let layout = store.layout();
    let (variant_offset, variants) = layout.variant_part.as_ref().unwrap();
    assert_eq!(variants.len(), 2);
    let value0 = (&layout.common[0], value_storage).try_into()?;
    let value1 = (&layout.common[1], value_storage).try_into()?;
    let variant0_value2 = (&variants[0][0]).try_into()?;
    let variant1_value2 = (&variants[1][0], value_storage).try_into()?;
    let variant_id = jbk::reader::builder::VariantIdProperty::new(*variant_offset);
    Ok(Builder {
        store,
        value0,
        value1,
        variant0_value2,
        variant1_value2,
        variant_id,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    // Let's read our container created in `simple_create.rs`
    let container = jbk::reader::Container::new("test.jbkm")?; // or "test.jbkm"
    let index = container.get_index_for_name("My own index")?;
    let builder = create_builder(
        index.get_store(&container.get_entry_storage())?,
        container.get_value_storage(),
    )?;

    {
        let entry = index.get_entry(&builder, 0.into())?;
        if let Entry::Variant0(entry) = entry {
            assert_eq!(entry.value0, Vec::from("Super"));
            assert_eq!(entry.value1, 50);
            // Let's print the content on stdout
            let reader = container.get_reader(entry.value2)?;
            std::io::copy(&mut reader.create_flux_all(), &mut std::io::stdout().lock())?;
        } else {
            panic!("We should have variant0")
        }
    }

    {
        let entry = index.get_entry(&builder, 1.into())?;
        if let Entry::Variant1(entry) = entry {
            assert_eq!(entry.value0, Vec::from("Mega"));
            assert_eq!(entry.value1, 42);
            assert_eq!(entry.value2, 5);
        } else {
            panic!("We should have variant1")
        }
    }

    {
        let entry = index.get_entry(&builder, 2.into())?;
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
