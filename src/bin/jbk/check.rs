use clap::Parser;
use jubako::{
    self as jbk,
    block_check::{check_blocks, CheckResult},
};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct Options {
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    #[clap(long)]
    check_blocks: bool,
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
    if options.check_blocks {
        for path in &options.infiles {
            match check_blocks(path)? {
                CheckResult::Ok(_) => println!("File {} is not corrupted", path.display()),
                CheckResult::Invalid(errors) => {
                    println!("File {} is corrupted", path.display());
                    for error in errors {
                        println!(" - {error}");
                    }
                }
            }
        }
    } else {
        for path in &options.infiles {
            check_one_file(path)?;
        }
    }
    Ok(())
}
