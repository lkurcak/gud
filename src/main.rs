mod branch;
mod fetch;
mod git;
mod log;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gud", about = "Interactive git helper", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Don't fetch from remotes in the background (or set `git config gud.autoFetch false`)
    #[arg(long, global = true)]
    no_fetch: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Interactively switch to or delete local branches
    B,
    /// Browse commits; soft/hard reset to one or edit its message
    L,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::B => branch::run(!cli.no_fetch)?,
        Command::L => log::run(!cli.no_fetch)?,
    };
    std::process::exit(code);
}
