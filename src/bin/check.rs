use clap::Parser;
use jubako as jbk;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "jbk-check")]
#[clap(author, version, about, long_about=None)]
struct Cli {
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,
}

fn check_one_file(path: PathBuf) -> jbk::Result<()> {
    let pack = jbk::tools::open_pack(&path)?;
    if pack.check()? {
        println!("Pack {} is ok", path.display());
    } else {
        println!("Pack {} s ko", path.display());
    }
    Ok(())
}

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    for path in args.infiles {
        check_one_file(path)?;
    }
    Ok(())
}
