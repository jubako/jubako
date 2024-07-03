use clap::{Parser, ValueEnum};
use jbk::reader::Explorable;
use jubako as jbk;
use std::path::PathBuf;

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Ron,
    Json,
    Yaml,
}

#[derive(Parser)]
pub struct Options {
    #[arg(value_parser)]
    infile: PathBuf,

    #[arg(value_parser, default_value = "")]
    key: String,

    #[arg(long, value_enum, default_value = "yaml")]
    format: Format,
}

pub fn run(options: Options) -> jbk::Result<()> {
    let pack = Box::new(jbk::tools::open_pack(&options.infile)?);
    let explorable: Option<Box<dyn Explorable>> = if options.key.is_empty() {
        Some(pack)
    } else {
        let (mut explorable, mut left) = pack.explore(&options.key)?;
        while let (Some(e), Some(l)) = (&explorable, left) {
            (explorable, left) = e.explore(l)?;
        }
        explorable
    };

    if let Some(explorable) = explorable {
        match options.format {
            Format::Ron => {
                let mut serializer = ron::ser::Serializer::new(
                    std::io::stdout(),
                    Some(ron::ser::PrettyConfig::new().struct_names(true)),
                )
                .unwrap();
                erased_serde::serialize(&explorable, &mut serializer).unwrap();
            }
            Format::Yaml => {
                let mut serializer = serde_yaml::Serializer::new(std::io::stdout());
                erased_serde::serialize(&explorable, &mut serializer).unwrap();
            }
            Format::Json => {
                let mut serializer = serde_json::Serializer::pretty(std::io::stdout());
                erased_serde::serialize(&explorable, &mut serializer).unwrap();
            }
        }
    } else {
        eprintln!("Key {} is not valid", options.key);
    }
    Ok(())
}
