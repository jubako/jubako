use jbk::reader::EntryTrait;
use jubako as jbk;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Let's read our container created in `simple_create.rs`
    let container = jbk::reader::Container::new("test.jbkm")?; // or "test.jbkm"
    let directory = container.get_directory_pack();
    let index = directory.get_index_from_name("My own index")?;
    let entry_storage = directory.create_entry_storage();
    let value_storage = directory.create_value_storage();
    let resolver = jbk::reader::Resolver::new(value_storage); // This is needed to get our info in the value_store
    let finder: jbk::reader::Finder<jbk::reader::AnySchema> = index.get_finder(&entry_storage)?; // To found our entries.

    {
        let entry = finder.get_entry(0.into())?;
        assert_eq!(entry.get_variant_id(), 0.into()); // We correctly have variant 0
        assert_eq!(
            resolver.resolve_to_vec(&entry.get_value(0.into())?)?,
            Vec::from("Super")
        );
        assert_eq!(
            resolver.resolve_to_unsigned(&entry.get_value(1.into())?),
            50
        );
        let value_2 = entry.get_value(2.into())?;
        let content_address = resolver.resolve_to_content(&value_2);
        // Let's print the content on stdout
        let reader = container.get_reader(content_address)?;
        std::io::copy(
            &mut reader.create_stream_all(),
            &mut std::io::stdout().lock(),
        )?;
    }

    {
        let entry = finder.get_entry(1.into())?;
        assert_eq!(entry.get_variant_id(), 1.into());
        assert_eq!(
            resolver.resolve_to_vec(&entry.get_value(0.into())?)?,
            Vec::from("Mega")
        );
        assert_eq!(
            resolver.resolve_to_unsigned(&entry.get_value(1.into())?),
            42
        );
        assert_eq!(resolver.resolve_to_unsigned(&entry.get_value(2.into())?), 5);
    }

    {
        let entry = finder.get_entry(2.into())?;
        assert_eq!(entry.get_variant_id(), 1.into());
        assert_eq!(
            resolver.resolve_to_vec(&entry.get_value(0.into())?)?,
            Vec::from("Hyper")
        );
        assert_eq!(
            resolver.resolve_to_unsigned(&entry.get_value(1.into())?),
            45
        );
        assert_eq!(resolver.resolve_to_unsigned(&entry.get_value(2.into())?), 2);
    }

    Ok(())
}
