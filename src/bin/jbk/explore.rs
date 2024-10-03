use clap::{Parser, ValueEnum};
use graphex::{explore, explore_to_string, Node};
use jubako as jbk;
use std::path::PathBuf;

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Json,
}

#[derive(Parser)]
pub struct Options {
    #[arg(value_parser)]
    infile: PathBuf,

    #[arg(value_parser, default_value = "")]
    key: String,

    #[arg(long, value_enum)]
    format: Option<Format>,
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let pack = jbk::tools::open_pack(&options.infile)?;

    match options.format {
        None => match explore_to_string(&pack, &options.key) {
            Ok(output) => println!("{}", output),
            Err(graphex::Error::Key(_key)) => {
                println!("Error, {} is not a valid key", options.key)
            }
            Err(graphex::Error::Other(e)) => return Err(e.downcast::<jbk::Error>().unwrap()),
            Err(graphex::Error::Fmt(_)) => unreachable!(),
        },
        Some(Format::Json) => {
            let mut serializer = serde_json::Serializer::pretty(std::io::stdout());
            let display = |n: &dyn Node| {
                erased_serde::serialize(n.serde().unwrap(), &mut serializer).unwrap()
            };
            match explore(&pack, &options.key, display) {
                Ok(_) => {}
                Err(graphex::Error::Key(_key)) => {
                    println!("Error, {} is not a valid key", options.key)
                }
                Err(graphex::Error::Other(e)) => return Err(e.downcast::<jbk::Error>().unwrap()),
                Err(graphex::Error::Fmt(_)) => unreachable!(),
            }
        }
    }
    Ok(())
}
