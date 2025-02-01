use clap::Parser;
use jbk::reader::PackLocatorTrait;
use jubako as jbk;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Parser)]
pub struct Options {
    #[arg(value_parser)]
    infile: PathBuf,

    #[arg(value_parser)]
    uuid: Option<String>,

    #[arg(value_parser)]
    new_location: Option<String>,
}

pub fn run(options: Options) -> jbk::Result<()> {
    let uuid = match options.uuid {
        None => None,
        Some(uuid) => match Uuid::parse_str(&uuid) {
            Ok(uuid) => Some(uuid),
            Err(_) => {
                eprintln!("{uuid} is not a valid uuid");
                return Ok(());
            }
        },
    };
    if let Some(location) = &options.new_location {
        if location.len() > 217 {
            eprintln!("{location} is too long. Only 217 bytes allowed");
            return Ok(());
        }

        if uuid.is_none() {
            eprintln!("You must specify a uuid if you want to change the location");
            return Ok(());
        }
        let uuid = uuid.unwrap();

        match jbk::tools::set_location(options.infile, uuid, location.as_str().into()) {
            Ok(None) => {
                eprintln!("Pack {uuid} is not in the manifest");
            }
            Ok(Some((pack_kind, old_location))) => {
                println!(
                    "Change {:?} pack {} location from `{}` to `{}`",
                    pack_kind, uuid, old_location, location
                );
            }

            Err(e) => {
                eprintln!("{e}");
            }
        };
    } else {
        let container = Arc::new(jbk::tools::open_pack(&options.infile)?);

        let manifest_pack_reader = container.get_manifest_pack_reader()?;
        if manifest_pack_reader.is_none() {
            eprintln!("No manifest pack in {}", options.infile.display());
            return Ok(());
        };
        let manifest_pack_reader = manifest_pack_reader.unwrap();
        let manifest = jbk::reader::ManifestPack::new(manifest_pack_reader)?;
        for pack_info in manifest.get_pack_infos() {
            if let Some(uuid) = uuid {
                if pack_info.uuid != uuid {
                    continue;
                }
            }

            let fs_locator = Arc::new(jbk::reader::FsLocator::new(
                options.infile.parent().unwrap().to_path_buf(),
            ));
            let locators: Vec<Arc<dyn jbk::reader::PackLocatorTrait>> = vec![container, fs_locator];
            let locator = Arc::new(jbk::reader::ChainedLocator::new(locators));
            match locator.locate(pack_info.uuid, &pack_info.pack_location)? {
                None => println!(
                    "{:?} pack {} has declared location `{}`",
                    pack_info.pack_kind, pack_info.uuid, pack_info.pack_location,
                ),
                Some(reader) => println!(
                    "{:?} pack {} (with declared location `{}`) is located in {}",
                    pack_info.pack_kind, pack_info.uuid, pack_info.pack_location, reader
                ),
            }
            break;
        }
    };
    Ok(())
}
