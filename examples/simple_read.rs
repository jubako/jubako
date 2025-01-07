use jbk::reader::builder::AnyBuilder;
use jbk::reader::{EntryTrait, Range};
use jubako as jbk;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Let's read our container created in `simple_create.rs` or `basic_creator.rs`

    let container = jbk::reader::Container::new("test.jbkm")?; // or "test.jbk" if created using basic_creator.rs
    let index = container
        .get_index_for_name("My own index")?
        .expect("'My own index' should be in the container");
    let builder = AnyBuilder::new(
        index.get_store(container.get_entry_storage())?,
        container.get_value_storage().as_ref(),
    )?;

    {
        let entry = index
            .get_entry(&builder, 0.into())?
            .expect("We have the entry 0");
        assert_eq!(entry.get_variant_id().unwrap(), Some(0.into())); // We correctly have variant 0
        assert_eq!(
            entry.get_value("AString")?.unwrap().as_vec()?,
            Vec::from("Super")
        );
        assert_eq!(entry.get_value("AInteger")?.unwrap().as_unsigned(), 50);
        let value_2 = entry.get_value("TheContent")?.unwrap();
        let content_address = value_2.as_content();
        // Let's print the content on stdout
        let region = container
            .get_bytes(content_address)?
            .and_then(|m| m.transpose())
            .expect("content_address should be valid")
            .unwrap();
        std::io::copy(&mut region.stream(), &mut std::io::stdout().lock())?;
    }

    {
        let entry = index
            .get_entry(&builder, 1.into())?
            .expect("We have the entry 1");
        assert_eq!(entry.get_variant_id().unwrap(), Some(1.into()));
        assert_eq!(
            entry.get_value("AString")?.unwrap().as_vec()?,
            Vec::from("Mega")
        );
        assert_eq!(entry.get_value("AInteger")?.unwrap().as_unsigned(), 42);
        assert_eq!(entry.get_value("AnotherInt")?.unwrap().as_unsigned(), 5);
    }

    {
        let entry = index
            .get_entry(&builder, 2.into())?
            .expect("We have the entry 2");

        assert_eq!(entry.get_variant_id().unwrap(), Some(1.into()));
        assert_eq!(
            entry.get_value("AString")?.unwrap().as_vec()?,
            Vec::from("Hyper")
        );
        assert_eq!(entry.get_value("AInteger")?.unwrap().as_unsigned(), 45);
        assert_eq!(entry.get_value("AnotherInt")?.unwrap().as_unsigned(), 2);
    }

    Ok(())
}
