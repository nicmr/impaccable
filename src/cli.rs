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
    Sync {
        /// Remove packages not tracked by your configuration
        #[arg(long)]
        remove_untracked: bool,
    },
    /// Add packages to specified group
    Add {
        #[arg(required=true, num_args=1..)]
        packages: Vec<String>,

        #[arg(short, long, required=true)]
        group: String,
    },
    /// Remove a package from specified group is using
    Remove {
        #[arg(required=true)]
        package: String,

        #[arg(short, long, required=true)]
        group: String,

        // TODO(low, UX): add active flag to just remove packages from active configuration if possible. make default?
    },
    #[command(subcommand)]
    Target(Target),

      /// Check what changes a Sync would apply
    Plan {
        /// Evaluate what changes sync with this flag would apply
        #[arg(long)]
        remove_untracked: bool
    },

    /// Get a package template for your Arch-based distro
    Template,

    /// Import packages existing on your system into your config
    Import,

    // Review / import 
    //    // untracked / trials

    #[command(subcommand)]
    Groups(Groups)
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

        #[arg(long)]
        force: bool,
    },
}

/// Manage Groups
#[derive(Subcommand)]
pub enum Groups {
    Ls,
}