use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about="A mildly declarative pacman wrapper", arg_required_else_help=true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<CliCommand>,

    #[arg(short, long, value_name="CONFIG_PATH")]
    pub config: Option<PathBuf>,

    #[arg(short, long, value_name="TARGET_PATH")]
    pub target: Option<PathBuf>
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// Dump the configuration file
    Config,
    /// Sync your target with the package configuration
    Sync,
    /// Add packages to specified group
    Add {
        #[arg(required=true, num_args=1..)]
        packages: Vec<String>,

        #[arg(short, long, required=true)]
        group: String,
    },
    /// Remove a package from groups your target is using
    Remove {
        #[arg(required=true)]
        package: String,
    },
    #[command(subcommand)]
    Target(Target),

    // Try, // should this be add --trial instead?

    /// Compare your target with the active package configuration
    Diff {
        /// Also output untracked packages
        #[arg(short, long)]
        untracked: bool
    },

    // Review / import 
    //    // untracked / trials
}

/// Interact with target configuration
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