mod cli;
mod declareman;


use clap::Parser;
use declareman::{DeclaremanConfig, PackageConfiguration, install_packages};
use std::{path::{Path}};
use anyhow::{Ok, Context};
use cli::{Cli, CliCommand};

fn main() -> std::result::Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let config_path = Path::new("./declareman/config.toml");
    let config_string = std::fs::read_to_string(config_path)?;
    let config : DeclaremanConfig = toml::from_str(&config_string)?;


    match &cli.command {
        Some(CliCommand::Config) => {
            println!("config: {:?}", config);
        },
        Some(CliCommand::Sync) => {
            let package_config = PackageConfiguration::try_parse(&config, config_path)?;
            install_packages(&package_config.groups, &config.root_group)?;
        }
        Some(CliCommand::Add { packages, group }) => {
            let mut package_config = PackageConfiguration::try_parse(&config, config_path)?;
            // if let Some(matching_group) = package_config.groups.get_mut(group) {
            //     matching_group.members.append(&mut packages.clone())
            // }

            let unique_packages = packages.clone().into_iter().collect();
            package_config.add_packages(unique_packages, group).context("Failed to add packages")?;
            println!("Added the following packages{:?}", packages);
        }
        None => {},
    }
    Ok(())
}




