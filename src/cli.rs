use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about="A mildly declarative pacman wrapper", arg_required_else_help=true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<CliCommand>
}

#[derive(Subcommand)]
pub enum CliCommand {
    #[command(about="Dump the configuration file")]
    Config,
    #[command(about="Sync your target with the package configuration")]
    Sync,
    #[command(about="Add packages to specified group")]
    Add {
        #[arg(required=true, num_args=1..)]
        packages: Vec<String>,

        #[arg(short, long, required=true)]
        group: String,
    },
    #[command(about="Remove a package from groups your target is using")]
    Remove {
        #[arg(required=true)]
        package: String,
    },
    #[command(subcommand)]
    Target(Target),

    // Try, // should this be add --trial instead?

    // Review / import 
    //    // untracked / trials
}

#[derive(Subcommand)]
pub enum Target {
    #[command(about="List all targets")]
    Ls,
    #[command(about="Get the active target")]
    Get,
    #[command(about="Set the active target")]
    Set {
        #[arg(required=true)]
        target: String,
    },
}