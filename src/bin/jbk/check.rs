use clap::Parser;
use jubako as jbk;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct Options {
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,
}

fn check_one_file(path: &Path) -> jbk::Result<()> {
    let pack = jbk::tools::open_pack(path)?;
    if pack.check()? {
        println!("Pack {} is ok", path.display());
    } else {
        println!("Pack {} s ko", path.display());
    }
    Ok(())
}

pub fn run(options: Options) -> jbk::Result<()> {
    for path in &options.infiles {
        check_one_file(&path)?;
    }
    Ok(())
}
