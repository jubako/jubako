use clap::{Parser, ValueEnum};
use jubako as jbk;
use std::path::PathBuf;

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Plain,
    #[cfg(feature = "explorable_serde")]
    Json,
}

#[derive(Parser)]
pub struct Options {
    #[arg(value_parser)]
    infile: PathBuf,

    #[arg(value_parser, default_value = "")]
    key: String,

    #[arg(long, value_enum, default_value = "plain")]
    format: Format,
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    yansi::whenever(yansi::Condition::STDOUT_IS_TTY);
    let pack = jbk::tools::open_pack(&options.infile)?;

    match options.format {
        Format::Plain => match graphex::explore_to_string(&pack, &options.key) {
            Ok(output) => println!("{}", output),
            Err(graphex::Error::Key(_key)) => {
                println!("Error, {} is not a valid key", options.key)
            }
            Err(graphex::Error::Other(e)) => return Err(e.downcast::<jbk::Error>().unwrap()),
            Err(graphex::Error::Fmt(_)) => unreachable!(),
        },
        #[cfg(feature = "explorable_serde")]
        Format::Json => {
            let mut serializer = serde_json::Serializer::pretty(std::io::stdout());
            let display = |n: &dyn graphex::Node| {
                erased_serde::serialize(n.serde().unwrap(), &mut serializer).unwrap()
            };
            match graphex::explore(&pack, &options.key, display) {
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
