mod branch;
mod git;
mod log;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gud", about = "Interactive git helper", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Interactively switch to or delete local branches
    #[command(visible_aliases = ["b", "switch"])]
    Branch,
    /// Browse commits; soft/hard reset to one or edit its message
    #[command(visible_alias = "l")]
    Log,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Branch => branch::run()?,
        Command::Log => log::run()?,
    };
    std::process::exit(code);
}
