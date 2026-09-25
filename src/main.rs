mod branch;
mod git;

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
    B,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::B => branch::run()?,
    };
    std::process::exit(code);
}
