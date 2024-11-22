mod check;
mod concat;
mod explore;
mod locate;

use clap::Parser;

const VERSION: &str = const_format::formatcp!(
    "{}{}",
    clap::crate_version!(),
    git_version::git_version!(
        args = ["--dirty=*", "--tags", "--always"],
        fallback = "",
        prefix = " (git:",
        suffix = ")"
    )
);

#[derive(Parser)]
#[clap(name="jbk", author, version, long_version=VERSION, about, long_about=None)]
struct Options {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Check(check::Options),
    Concat(concat::Options),
    Explore(explore::Options),
    Locate(locate::Options),
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse();

    match options.command {
        Commands::Check(options) => check::run(options)?,
        Commands::Concat(options) => concat::run(options)?,
        Commands::Explore(options) => explore::run(options)?,
        Commands::Locate(options) => locate::run(options)?,
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
    }
}
