use clap::Parser;
use jbk::creator::OutStream;
use jbk::reader::{ManifestPackHeader, PackLocatorTrait, Producable};
use jubako as jbk;
use std::io::{Seek, SeekFrom};
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
        if location.as_bytes().len() > 217 {
            eprintln!("{location} is too long. Only 217 bytes allowed");
            return Ok(());
        }
    }
    let container = Arc::new(jbk::tools::open_pack(&options.infile)?);

    let manifest_pack_reader = container.get_manifest_pack_reader()?;
    let fs_locator = Arc::new(jbk::reader::FsLocator::new(
        options.infile.parent().unwrap().to_path_buf(),
    ));
    let locators: Vec<Arc<dyn jbk::reader::PackLocatorTrait>> = vec![container, fs_locator];
    let locator = Arc::new(jbk::reader::ChainedLocator(locators));

    if manifest_pack_reader.is_none() {
        eprintln!("No manifest pack in {}", options.infile.display());
        return Ok(());
    };
    let manifest_pack_reader = manifest_pack_reader.unwrap();
    let mut flux = manifest_pack_reader.create_flux_all();
    let header = ManifestPackHeader::produce(&mut flux)?;
    flux.seek(header.packs_offset());
    for _i in header.pack_count {
        let pack_info = jbk::reader::PackInfo::produce(&mut flux)?;
        if let Some(uuid) = uuid {
            if pack_info.uuid != uuid {
                continue;
            }
        }
        let location = String::from_utf8_lossy(&pack_info.pack_location);

        if let Some(new_location) = options.new_location {
            let location_offset = flux.global_offset() - jbk::Size::new(218);
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&options.infile)?;
            file.seek(SeekFrom::Start(location_offset.into_u64()))?;
            file.ser_callable(&|ser| -> std::io::Result<()> {
                jbk::PString::serialize_string_padded(new_location.as_bytes(), 217, ser)?;
                Ok(())
            })?;
            println!(
                "Change {:?} pack {} location from `{}` to `{}`",
                pack_info.pack_kind, pack_info.uuid, location, new_location
            );
            break;
        } else {
            match locator.locate(pack_info.uuid, &pack_info.pack_location)? {
                None => println!(
                    "{:?} pack {} has declared location `{}`",
                    pack_info.pack_kind, pack_info.uuid, location,
                ),
                Some(reader) => println!(
                    "{:?} pack {} (with declared location `{}`) is located in {}",
                    pack_info.pack_kind, pack_info.uuid, location, reader
                ),
            }
        }
    }

    Ok(())
}
