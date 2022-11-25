use jbk::reader::EntryTrait;
use jubako as jbk;
use std::error::Error;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn Error>> {
    // Let's read our container created in `simple_create.rs`
    let container = jbk::reader::Container::new("test.jbkm")?; // or "test.jbkm"
    let directory = container.get_directory_pack()?;
    let index = directory.get_index_from_name("My own index")?;
    let resolver = directory.get_resolver(); // This is needed to get our info in the value_store
    let finder = index.get_finder(Rc::clone(&resolver)); // To found our entries.

    {
        let entry = finder.get_entry(0.into())?;
        assert_eq!(entry.get_variant_id(), 0); // We correctly have variant 0
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
        assert_eq!(entry.get_variant_id(), 1);
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
        assert_eq!(entry.get_variant_id(), 1);
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
