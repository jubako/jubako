use jubako as jbk;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Options {
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    // Out file
    #[clap(short, long, value_parser)]
    outfile: jbk::Utf8PathBuf,
}

pub fn run(options: Options) -> jbk::Result<()> {
    jbk::tools::concat(&options.infiles, options.outfile)
}
