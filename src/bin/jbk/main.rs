mod check;
mod concat;
mod explore;

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
    Explore(explore::Options),
}

fn run() -> jbk::Result<()> {
    let options = Options::parse();

    match options.command {
        Commands::Check(options) => check::run(options),
        Commands::Concat(options) => concat::run(options),
        Commands::Explore(options) => explore::run(options),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
    }
}
