use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about="A mildly declarative pacman wrapper", arg_required_else_help=true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<CliCommand>
}

#[derive(Subcommand)]
pub enum CliCommand {
    Config,
    Sync,
    Add {
        #[arg(required=true, num_args=1..)]
        packages: Vec<String>,

        #[arg(short, long, required=true)]
        group: String,
    },
    // Remove,
    // Try,
}