use jubako as jbk;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "jbk-concat")]
#[clap(author, version, about, long_about=None)]
struct Cli {
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    // Out file
    #[clap(short, long, value_parser)]
    outfile: PathBuf,
}

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    jbk::concat(&args.infiles, args.outfile)
}
