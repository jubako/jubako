mod check;
mod concat;

use clap::Parser;
use jubako as jbk;

#[derive(Parser)]
#[clap(name="jbk", author, version, about, long_about=None)]
struct Options {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Check(check::Options),
    Concat(concat::Options),
}

fn run() -> jbk::Result<()> {
    let options = Options::parse();

    match options.command {
        Commands::Check(options) => check::run(options),
        Commands::Concat(options) => concat::run(options),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
    }
}
