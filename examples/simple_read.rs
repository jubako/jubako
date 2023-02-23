use jbk::reader::schema::SchemaTrait;
use jbk::reader::EntryTrait;
use jubako as jbk;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Let's read our container created in `simple_create.rs`
    let container = jbk::reader::Container::new("test.jbkm")?; // or "test.jbkm"
    let index = container.get_index_for_name("My own index")?;
    let builder = jbk::reader::AnySchema::create_builder(
        index.get_store(&container.get_entry_storage())?,
        container.get_value_storage(),
    )?;
    let finder: jbk::reader::Finder<jbk::reader::AnySchema> = index.get_finder(builder)?; // To found our entries.

    {
        let entry = finder.get_entry(0.into())?;
        assert_eq!(entry.get_variant_id().unwrap(), Some(0.into())); // We correctly have variant 0
        assert_eq!(entry.get_value(0.into())?.as_vec()?, Vec::from("Super"));
        assert_eq!(entry.get_value(1.into())?.as_unsigned(), 50);
        let value_2 = entry.get_value(2.into())?;
        let content_address = value_2.as_content();
        // Let's print the content on stdout
        let reader = container.get_reader(content_address)?;
        std::io::copy(&mut reader.create_flux_all(), &mut std::io::stdout().lock())?;
    }

    {
        let entry = finder.get_entry(1.into())?;
        assert_eq!(entry.get_variant_id().unwrap(), Some(1.into()));
        assert_eq!(entry.get_value(0.into())?.as_vec()?, Vec::from("Mega"));
        assert_eq!(entry.get_value(1.into())?.as_unsigned(), 42);
        assert_eq!(entry.get_value(2.into())?.as_unsigned(), 5);
    }

    {
        let entry = finder.get_entry(2.into())?;
        assert_eq!(entry.get_variant_id().unwrap(), Some(1.into()));
        assert_eq!(entry.get_value(0.into())?.as_vec()?, Vec::from("Hyper"));
        assert_eq!(entry.get_value(1.into())?.as_unsigned(), 45);
        assert_eq!(entry.get_value(2.into())?.as_unsigned(), 2);
    }

    Ok(())
}
