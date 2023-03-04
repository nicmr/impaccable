mod cli;
mod declareman;


use clap::Parser;
use declareman::{DeclaremanConfig, PackageConfiguration, install_packages, ActiveTarget};
use directories::ProjectDirs;
use std::{path::{Path, PathBuf}, fs, env};
use anyhow::{Context};
use cli::{Cli, CliCommand, Target};

fn main() -> std::result::Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let config_path = {
        if let Some(cli_config_override) = cli.config {
            cli_config_override
        }
        else if let Ok(env_config_override)= env::var("DECLAREMAN_CONFIG") {
            PathBuf::from(env_config_override)
        } else {
            // TODO: replace unwrap
            // TODO: check if directories is even needed, as this only runs on Linux anyway, and its main benefit is being cross-platform
            let default_project_dirs = ProjectDirs::from("dev.nicolasmohr.declareman", "Declareman Devs", "declareman")
            .unwrap();
            let mut default_config_path = default_project_dirs.config_dir().to_path_buf();
            default_config_path.push("config.toml");
            default_config_path
        }
    };

    // let config_path = Path::new("./declareman/config.toml");


    let config : DeclaremanConfig = {
        let config_string = fs::read_to_string(&config_path)?;
        toml::from_str(&config_string).context("Failed to parse declareman configuration")?
    };
    let mut package_config = PackageConfiguration::parse(&config, &config_path).context("Failed to parse package configuration")?;
    
    let active_target_path = Path::new("./declareman/active-target.toml");
    let mut active_target = ActiveTarget::parse(active_target_path).context("Failed to parse active target")?;

    match &cli.command {
        Some(CliCommand::Config) => {
            println!("config: {:?}", config);
        },
        Some(CliCommand::Sync) => {
            install_packages(&package_config.groups, &config.root_group)?;
        }
        Some(CliCommand::Add { packages, group }) => {
            let unique_packages = packages.clone().into_iter().collect();
            package_config.add_packages(unique_packages, group).context("Failed to add packages")?;
            println!("Added the following packages{:?}", packages);
        }
        Some(CliCommand::Remove { package }) => {
            let removed_from_groups = package_config.remove_package(package);
            println!("Removed from the following groups: {:?}", removed_from_groups)
        }
        Some(CliCommand::Target(subcommand)) => {
            match subcommand {
                Target::Ls => {
                    let active_target = active_target.target();
                    for (name, _) in config.targets {
                        if &name == active_target {
                            println!("{} (active)", name);
                        } else {
                            println!("{}", name);
                        }
                    }
                },
                Target::Get => {
                    println!("{}", active_target.target());
                },
                Target::Set { target } => {
                    active_target.set_target(target.clone(), active_target_path).context("Failed to set active target")?;
                },
            }
        }
        Some(CliCommand::Diff) => {
            let pacman_installed = declareman::pacman_query_installed().context("Failed to query installed packages")?;
            for entry in pacman_installed {
                println!("{}", entry)
            }
        }
        None => {},
    }
    Ok(())
}




